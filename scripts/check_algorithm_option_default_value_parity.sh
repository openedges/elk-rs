#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
REPORT_FILE="${1:-parity/algorithm_option_default_value_parity.md}"
STRICT_MODE="${ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_STRICT:-false}"
KEEP_TMP="${ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_KEEP_TMP:-false}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-option-default-value-parity.XXXXXX")"
cleanup_tmp() {
    if [ "$KEEP_TMP" = "true" ]; then
        echo "keeping temp directory: $tmp_dir" >&2
        return
    fi
    rm -rf "$tmp_dir"
}
trap cleanup_tmp EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

java_supported_option_ids_file="$tmp_dir/java_supported_option_ids.tsv"
java_const_map_file="$tmp_dir/java_const_map.tsv"
java_option_defaults_raw_file="$tmp_dir/java_option_defaults.raw.tsv"
java_option_defaults_file="$tmp_dir/java_option_defaults.tsv"
rust_option_defaults_raw_file="$tmp_dir/rust_option_defaults.raw.tsv"
rust_option_defaults_file="$tmp_dir/rust_option_defaults.tsv"
java_filtered_file="$tmp_dir/java_filtered.tsv"
rust_filtered_file="$tmp_dir/rust_filtered.tsv"
java_ids_file="$tmp_dir/java_ids.tsv"
rust_ids_file="$tmp_dir/rust_ids.tsv"
missing_ids_file="$tmp_dir/missing_ids.tsv"
extra_ids_file="$tmp_dir/extra_ids.tsv"
value_mismatches_file="$tmp_dir/value_mismatches.tsv"
uncomparable_file="$tmp_dir/uncomparable.tsv"

# Java algorithm option ids from addOptionSupport(algo, option, defaultExpr).
(rg --files "$JAVA_SOURCES_ROOT" -g '*Options.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            function trim_ws(text) {
                gsub(/^[[:space:]]+/, "", text)
                gsub(/[[:space:]]+$/, "", text)
                return text
            }
            function normalize_option_id(id) {
                if (id ~ /^org\.eclipse\.elk\.alg\.layered\./) {
                    sub(/^org\.eclipse\.elk\.alg\.layered\./, "org.eclipse.elk.layered.", id)
                }
                return id
            }
            function flush_call(text,    call, first, second, rest) {
                call = text
                if (!match(call, /"org\.eclipse\.elk[^"]+"/)) {
                    return
                }
                first = substr(call, RSTART + 1, RLENGTH - 2)
                rest = substr(call, RSTART + RLENGTH)
                if (!match(rest, /"org\.eclipse\.elk[^"]+"/)) {
                    return
                }
                second = substr(rest, RSTART + 1, RLENGTH - 2)
                print normalize_option_id(second)
            }
            {
                if (!in_call && $0 ~ /registry\.addOptionSupport[[:space:]]*\(/) {
                    in_call = 1
                    call_text = $0
                } else if (in_call) {
                    call_text = call_text " " $0
                }
                if (in_call && $0 ~ /\)[[:space:]]*;/) {
                    flush_call(call_text)
                    in_call = 0
                    call_text = ""
                }
            }
        ' "$file"
    done | sort -u > "$java_supported_option_ids_file"

# Java constant map: CONST_NAME / ClassName.CONST_NAME -> expression
(rg --files "$JAVA_SOURCES_ROOT" -g '*.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            function trim_ws(text) {
                gsub(/^[[:space:]]+/, "", text)
                gsub(/[[:space:]]+$/, "", text)
                return text
            }
            /class [A-Za-z_][A-Za-z0-9_]*/ {
                if (class_name == "") {
                    class_name = $0
                    sub(/^.*class[[:space:]]+/, "", class_name)
                    sub(/[^A-Za-z0-9_].*$/, "", class_name)
                    class_name = trim_ws(class_name)
                }
            }
            /[[:space:]]static final [^=]*[A-Z0-9_]+[[:space:]]*=/ {
                name = $0
                sub(/^.*static final[[:space:]]+[^[:space:]]+[[:space:]]+/, "", name)
                sub(/[[:space:]]*=.*$/, "", name)
                name = trim_ws(name)
                if (name !~ /^[A-Z0-9_]+$/) {
                    next
                }
                rhs = $0
                sub(/^.*=[[:space:]]*/, "", rhs)
                sub(/[[:space:]]*;[[:space:]]*$/, "", rhs)
                rhs = trim_ws(rhs)
                print name "\t" rhs
                if (class_name != "") {
                    print class_name "." name "\t" rhs
                }
            }
        ' "$file"
    done | sort -u > "$java_const_map_file"

# Java option-id -> normalized default value from LayoutOptionData.Builder defaultValue(...).
(rg --files "$JAVA_SOURCES_ROOT" -g '*.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk -v const_map_file="$java_const_map_file" '
            BEGIN {
                while ((getline line < const_map_file) > 0) {
                    split(line, parts, "\t")
                    if (parts[1] != "" && parts[2] != "") {
                        const_expr[parts[1]] = parts[2]
                    }
                }
                close(const_map_file)
            }
            function trim_ws(text) {
                gsub(/^[[:space:]]+/, "", text)
                gsub(/[[:space:]]+$/, "", text)
                return text
            }
            function remove_ws(text) {
                gsub(/[[:space:]]+/, "", text)
                return text
            }
            function normalize_token(text,    out) {
                out = tolower(text)
                gsub(/[^a-z0-9]/, "", out)
                return out
            }
            function normalize_number(text, value) {
                value = tolower(text)
                gsub(/_/, "", value)
                sub(/[dfl]$/, "", value)
                return value
            }
            function normalize_scalar(text,    value) {
                value = normalize_number(text)
                if (value !~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                    return ""
                }
                if (value !~ /[eE]/ && value ~ /\./) {
                    while (value ~ /0$/) {
                        sub(/0$/, "", value)
                    }
                    sub(/\.$/, "", value)
                }
                if (value == "-0") {
                    value = "0"
                }
                return value
            }
            function strip_parens(text, out) {
                out = trim_ws(text)
                while (out ~ /^\([^()]+\)$/) {
                    sub(/^\(/, "", out)
                    sub(/\)$/, "", out)
                    out = trim_ws(out)
                }
                return out
            }
            function normalize_option_id(id) {
                if (id ~ /^org\.eclipse\.elk\.alg\.layered\./) {
                    sub(/^org\.eclipse\.elk\.alg\.layered\./, "org.eclipse.elk.layered.", id)
                }
                return id
            }
            function read_option_id(text, value) {
                if (!match(text, /"org\.eclipse\.elk[^"]+"/)) {
                    return ""
                }
                value = substr(text, RSTART + 1, RLENGTH - 2)
                return normalize_option_id(value)
            }
            function resolve_expr(expr,    compact, depth, n, parts, key, scoped_key) {
                compact = trim_ws(expr)
                depth = 0
                while (depth < 30) {
                    if (compact ~ /^[A-Za-z_][A-Za-z0-9_.]*\.[A-Z][A-Z0-9_]*$/) {
                        break
                    }
                    key = ""
                    if (compact in const_expr) {
                        key = compact
                    } else {
                        n = split(compact, parts, /\./)
                        scoped_key = parts[n - 1] "." parts[n]
                        if (scoped_key in const_expr) {
                            key = scoped_key
                        } else if (parts[n] in const_expr) {
                            key = parts[n]
                        }
                    }
                    if (key == "" && compact ~ /^[A-Za-z_][A-Za-z0-9_]*$/ && class_name != "") {
                        scoped_key = class_name "." compact
                        if (scoped_key in const_expr) {
                            key = scoped_key
                        }
                    }
                    if (key == "") {
                        break
                    }
                    compact = trim_ws(const_expr[key])
                    depth++
                }
                return compact
            }
            function parse_java_int_token(text,    token) {
                token = strip_parens(text)
                sub(/^Integer\.valueOf\(/, "", token)
                sub(/\)$/, "", token)
                token = strip_parens(token)
                token = normalize_number(token)
                if (token ~ /^[-+]?[0-9]+$/) {
                    return token
                }
                return ""
            }
            function parse_java_int_list(expr,    compact, body, n, parts, i, token, out) {
                compact = expr
                gsub(/[[:space:]]+/, "", compact)
                if (compact ~ /^Collections\.<Integer>unmodifiableList\(CollectionLiterals\.<Integer>newArrayList\(/) {
                    body = compact
                    sub(/^Collections\.<Integer>unmodifiableList\(CollectionLiterals\.<Integer>newArrayList\(/, "", body)
                    sub(/\)\)$/, "", body)
                } else if (compact ~ /^CollectionLiterals\.<Integer>newArrayList\(/) {
                    body = compact
                    sub(/^CollectionLiterals\.<Integer>newArrayList\(/, "", body)
                    sub(/\)$/, "", body)
                } else if (compact ~ /^Arrays\.asList\(/) {
                    body = compact
                    sub(/^Arrays\.asList\(/, "", body)
                    sub(/\)$/, "", body)
                } else {
                    return ""
                }
                if (body == "") {
                    return ""
                }
                n = split(body, parts, /,/)
                out = ""
                for (i = 1; i <= n; i++) {
                    token = parse_java_int_token(parts[i])
                    if (token == "") {
                        return ""
                    }
                    if (out != "") {
                        out = out ","
                    }
                    out = out token
                }
                return out
            }
            function parse_java_object(expr,    compact, args, n, parts, i, value, out) {
                compact = expr
                gsub(/[[:space:]]+/, "", compact)
                if (compact ~ /^newKVectorChain\(\)$/) {
                    return "kvectorchain:new"
                }
                if (compact ~ /^newKVector\(/) {
                    args = compact
                    sub(/^newKVector\(/, "", args)
                    sub(/\)$/, "", args)
                    if (args == "") {
                        return "kvector:0,0"
                    }
                    n = split(args, parts, /,/)
                    if (n == 2) {
                        parts[1] = normalize_scalar(parts[1])
                        parts[2] = normalize_scalar(parts[2])
                        if (parts[1] ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/ \
                                && parts[2] ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                            return "kvector:" parts[1] "," parts[2]
                        }
                    }
                    return ""
                }
                if (compact ~ /^newElkPadding\(/) {
                    args = compact
                    sub(/^newElkPadding\(/, "", args)
                    sub(/\)$/, "", args)
                    if (args == "") {
                        return "elkpadding:0"
                    }
                    n = split(args, parts, /,/)
                    out = ""
                    for (i = 1; i <= n; i++) {
                        value = normalize_scalar(parts[i])
                        if (value !~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                            return ""
                        }
                        if (out != "") {
                            out = out ","
                        }
                        out = out value
                    }
                    return "elkpadding:" out
                }
                if (compact ~ /^newElkMargin\(/) {
                    args = compact
                    sub(/^newElkMargin\(/, "", args)
                    sub(/\)$/, "", args)
                    if (args == "") {
                        return "elkmargin:0"
                    }
                    n = split(args, parts, /,/)
                    if (n == 1) {
                        value = normalize_scalar(parts[1])
                        if (value ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                            return "elkmargin:" value
                        }
                        return ""
                    }
                    out = ""
                    for (i = 1; i <= n; i++) {
                        value = normalize_scalar(parts[i])
                        if (value !~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                            return ""
                        }
                        if (out != "") {
                            out = out ","
                        }
                        out = out value
                    }
                    return "elkmargin:" out
                }
                return ""
            }
            function classify(expr,    compact, token, n, parts, out, list_value, object_value) {
                compact = strip_parens(trim_ws(expr))
                out_raw = compact
                if (compact ~ /^[A-Za-z_][A-Za-z0-9_.]*\.[A-Z][A-Z0-9_]*$/) {
                    token = compact
                    sub(/^.*\./, "", token)
                    out_class = "enum"
                    out_value = normalize_token(token)
                    return
                }
                list_value = parse_java_int_list(compact)
                if (list_value != "") {
                    out_class = "list"
                    out_value = list_value
                    return
                }
                object_value = parse_java_object(compact)
                if (object_value != "") {
                    out_class = "object"
                    out_value = object_value
                    return
                }

                compact = resolve_expr(compact)
                compact = strip_parens(compact)
                out_raw = compact

                list_value = parse_java_int_list(compact)
                if (list_value != "") {
                    out_class = "list"
                    out_value = list_value
                    return
                }
                object_value = parse_java_object(compact)
                if (object_value != "") {
                    out_class = "object"
                    out_value = object_value
                    return
                }

                if (compact == "" || compact == "null") {
                    out_class = "null"
                    out_value = "null"
                    return
                }
                if (compact ~ /^(Boolean|java\.lang\.Boolean)\.valueOf\((true|false)\)$/) {
                    token = compact
                    sub(/^.*\(/, "", token)
                    sub(/\)$/, "", token)
                    out_class = "bool"
                    out_value = token
                    return
                }
                if (compact ~ /^(Integer|Long|Float|Double|java\.lang\.(Integer|Long|Float|Double))\.valueOf\(/) {
                    token = compact
                    sub(/^.*\.valueOf\(/, "", token)
                    sub(/\)$/, "", token)
                    token = strip_parens(token)
                    if (normalize_number(token) ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                        out_class = "number"
                        out_value = normalize_number(token)
                        return
                    }
                }
                if (compact == "Integer.MAX_VALUE" || compact == "Long.MAX_VALUE") {
                    out_class = "enum"
                    out_value = "max"
                    return
                }
                if (compact ~ /^EnumSet\..*noneOf\(/) {
                    out_class = "enum"
                    out_value = "noneof"
                    return
                }
                if (compact == "true" || compact == "false") {
                    out_class = "bool"
                    out_value = compact
                    return
                }
                if (compact ~ /^"[^"]*"$/) {
                    out_class = "string"
                    out_value = substr(compact, 2, length(compact) - 2)
                    return
                }
                if (normalize_number(compact) ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                    out_class = "number"
                    out_value = normalize_scalar(compact)
                    return
                }

                token = compact
                n = split(token, parts, /\./)
                token = parts[n]
                if (token ~ /^[A-Z0-9_]+$/ || compact ~ /\./) {
                    out_class = "enum"
                    out_value = normalize_token(token)
                    return
                }

                out_class = "symbol"
                out_value = normalize_token(compact)
            }
            function flush_option() {
                if (option_id != "") {
                    classify(option_default_expr)
                    print option_id "\t" out_class "\t" out_value "\t" out_raw
                }
                in_option = 0
                option_id = ""
                option_default_expr = "null"
            }

            /[[:space:]]static final [^=]*[A-Z0-9_]+[[:space:]]*=/ {
                name = $0
                sub(/^.*static final[[:space:]]+[^[:space:]]+[[:space:]]+/, "", name)
                sub(/[[:space:]]*=.*$/, "", name)
                name = trim_ws(name)
                if (name !~ /^[A-Z0-9_]+$/) {
                    next
                }
                rhs = $0
                sub(/^.*=[[:space:]]*/, "", rhs)
                sub(/[[:space:]]*;[[:space:]]*$/, "", rhs)
                const_expr[name] = trim_ws(rhs)
                if (class_name != "") {
                    const_expr[class_name "." name] = trim_ws(rhs)
                }
            }

            /class [A-Za-z_][A-Za-z0-9_]*/ {
                if (class_name == "") {
                    class_name = $0
                    sub(/^.*class[[:space:]]+/, "", class_name)
                    sub(/[^A-Za-z0-9_].*$/, "", class_name)
                    class_name = trim_ws(class_name)
                }
            }

            /LayoutOptionData\.Builder\(\)/ {
                if (in_option == 1) {
                    flush_option()
                }
                in_option = 1
                option_id = ""
                option_default_expr = "null"
                next
            }

            in_option == 1 && /\.id\("org\.eclipse\.elk/ {
                option_id = read_option_id($0)
                next
            }

            in_option == 1 && /\.defaultValue[[:space:]]*\(/ {
                expr = $0
                sub(/^.*\.defaultValue[[:space:]]*\(/, "", expr)
                sub(/\)[[:space:]]*.*$/, "", expr)
                option_default_expr = trim_ws(expr)
                next
            }

            in_option == 1 && /\.create\(\)/ {
                flush_option()
            }

            END {
                if (in_option == 1) {
                    flush_option()
                }
            }
        ' "$file"
    done | sort -u > "$java_option_defaults_raw_file"

awk -F '\t' '
    function score(cls) {
        if (cls == "number" || cls == "bool" || cls == "string" || cls == "enum" || cls == "list" || cls == "object") {
            return 4
        }
        if (cls == "symbol") {
            return 3
        }
        if (cls == "null") {
            return 2
        }
        return 1
    }
    {
        id = $1
        cls = $2
        if (!(id in best_score) || score(cls) > best_score[id]) {
            best_score[id] = score(cls)
            best_line[id] = $0
        }
    }
    END {
        for (id in best_line) {
            print best_line[id]
        }
    }
' "$java_option_defaults_raw_file" | sort -u > "$java_option_defaults_file"

# Rust option-id -> normalized default value from Property::with_default/new in *options.rs.
(rg --files "$RUST_SOURCES_ROOT" -g '*options.rs' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            function trim_ws(text) {
                gsub(/^[[:space:]]+/, "", text)
                gsub(/[[:space:]]+$/, "", text)
                return text
            }
            function remove_ws(text) {
                gsub(/[[:space:]]+/, "", text)
                return text
            }
            function normalize_token(text,    out) {
                out = tolower(text)
                gsub(/[^a-z0-9]/, "", out)
                return out
            }
            function normalize_number(text, value) {
                value = tolower(text)
                gsub(/_/, "", value)
                sub(/_[iu](8|16|32|64|128|size)$/, "", value)
                sub(/_f(32|64)$/, "", value)
                return value
            }
            function normalize_scalar(text,    value) {
                value = normalize_number(text)
                if (value !~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                    return ""
                }
                if (value !~ /[eE]/ && value ~ /\./) {
                    while (value ~ /0$/) {
                        sub(/0$/, "", value)
                    }
                    sub(/\.$/, "", value)
                }
                if (value == "-0") {
                    value = "0"
                }
                return value
            }
            function normalize_option_id(id) {
                if (id ~ /^org\.eclipse\.elk\.alg\.layered\./) {
                    sub(/^org\.eclipse\.elk\.alg\.layered\./, "org.eclipse.elk.layered.", id)
                }
                return id
            }
            function update_static_depth(text, tmp) {
                tmp = text
                static_paren_depth += gsub(/\(/, "", tmp)
                tmp = text
                static_paren_depth -= gsub(/\)/, "", tmp)
                tmp = text
                static_brace_depth += gsub(/\{/, "", tmp)
                tmp = text
                static_brace_depth -= gsub(/\}/, "", tmp)
            }
            function extract_call_args(text, marker,    start, i, ch, in_str, esc, depth, out) {
                marker = marker "("
                start = index(text, marker)
                if (start == 0) {
                    return ""
                }
                i = start + length(marker)
                in_str = 0
                esc = 0
                depth = 1
                out = ""
                while (i <= length(text)) {
                    ch = substr(text, i, 1)
                    if (in_str == 1) {
                        out = out ch
                        if (esc == 1) {
                            esc = 0
                        } else if (ch == "\\") {
                            esc = 1
                        } else if (ch == "\"") {
                            in_str = 0
                        }
                    } else {
                        if (ch == "\"") {
                            in_str = 1
                            out = out ch
                        } else if (ch == "(") {
                            depth++
                            out = out ch
                        } else if (ch == ")") {
                            depth--
                            if (depth == 0) {
                                return out
                            }
                            out = out ch
                        } else {
                            out = out ch
                        }
                    }
                    i++
                }
                return out
            }
            function split_next_arg(text,    depth_paren, depth_brace, depth_brack, in_str, esc, i, ch) {
                depth_paren = 0
                depth_brace = 0
                depth_brack = 0
                in_str = 0
                esc = 0
                for (i = 1; i <= length(text); i++) {
                    ch = substr(text, i, 1)
                    if (in_str == 1) {
                        if (esc == 1) {
                            esc = 0
                        } else if (ch == "\\") {
                            esc = 1
                        } else if (ch == "\"") {
                            in_str = 0
                        }
                        continue
                    }
                    if (ch == "\"") {
                        in_str = 1
                        continue
                    }
                    if (ch == "(") {
                        depth_paren++
                        continue
                    }
                    if (ch == ")") {
                        if (depth_paren > 0) {
                            depth_paren--
                        }
                        continue
                    }
                    if (ch == "{") {
                        depth_brace++
                        continue
                    }
                    if (ch == "}") {
                        if (depth_brace > 0) {
                            depth_brace--
                        }
                        continue
                    }
                    if (ch == "[") {
                        depth_brack++
                        continue
                    }
                    if (ch == "]") {
                        if (depth_brack > 0) {
                            depth_brack--
                        }
                        continue
                    }
                    if (ch == "," && depth_paren == 0 && depth_brace == 0 && depth_brack == 0) {
                        parsed_arg = trim_ws(substr(text, 1, i - 1))
                        parsed_rest = trim_ws(substr(text, i + 1))
                        return
                    }
                }
                parsed_arg = trim_ws(text)
                parsed_rest = ""
            }
            function read_quoted(text, value) {
                if (!match(text, /"[^"]*"/)) {
                    return ""
                }
                value = substr(text, RSTART + 1, RLENGTH - 2)
                return value
            }
            function strip_parens(text, out) {
                out = trim_ws(text)
                while (out ~ /^\([^()]+\)$/) {
                    sub(/^\(/, "", out)
                    sub(/\)$/, "", out)
                    out = trim_ws(out)
                }
                return out
            }
            function parse_rust_int_list(expr,    compact, body, n, parts, i, token, out) {
                compact = expr
                gsub(/[[:space:]]+/, "", compact)
                if (compact !~ /^vec!\[/) {
                    return ""
                }
                body = compact
                sub(/^vec!\[/, "", body)
                sub(/\]$/, "", body)
                if (body == "") {
                    return ""
                }
                n = split(body, parts, /,/)
                out = ""
                for (i = 1; i <= n; i++) {
                    token = normalize_number(parts[i])
                    if (token !~ /^[-+]?[0-9]+$/) {
                        return ""
                    }
                    if (out != "") {
                        out = out ","
                    }
                    out = out token
                }
                return out
            }
            function parse_rust_object(expr,    compact, args, n, parts, i, value, out) {
                compact = expr
                gsub(/[[:space:]]+/, "", compact)
                if (compact ~ /^KVectorChain::new\(\)$/) {
                    return "kvectorchain:new"
                }
                if (compact ~ /^KVector::new\(/) {
                    args = compact
                    sub(/^KVector::new\(/, "", args)
                    sub(/\)$/, "", args)
                    if (args == "") {
                        return "kvector:0,0"
                    }
                    n = split(args, parts, /,/)
                    if (n == 2) {
                        parts[1] = normalize_scalar(parts[1])
                        parts[2] = normalize_scalar(parts[2])
                        if (parts[1] ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/ \
                                && parts[2] ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                            return "kvector:" parts[1] "," parts[2]
                        }
                    }
                    return ""
                }
                if (compact ~ /^ElkPadding::with_any\(/) {
                    args = compact
                    sub(/^ElkPadding::with_any\(/, "", args)
                    sub(/\)$/, "", args)
                    value = normalize_scalar(args)
                    if (value ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                        return "elkpadding:" value
                    }
                    return ""
                }
                if (compact ~ /^ElkPadding::new\(\)$/) {
                    return "elkpadding:0"
                }
                if (compact ~ /^ElkMargin::new\(\)$/) {
                    return "elkmargin:0"
                }
                return ""
            }
            function classify_expr(expr,    compact, token, n, parts, list_value, object_value) {
                compact = trim_ws(expr)
                compact = strip_parens(compact)
                out_raw = compact

                list_value = parse_rust_int_list(compact)
                if (list_value != "") {
                    out_class = "list"
                    out_value = list_value
                    return
                }
                object_value = parse_rust_object(compact)
                if (object_value != "") {
                    out_class = "object"
                    out_value = object_value
                    return
                }

                if (compact == "" || compact == "None") {
                    out_class = "null"
                    out_value = "null"
                    return
                }
                if (compact == "true" || compact == "false") {
                    out_class = "bool"
                    out_value = compact
                    return
                }
                if (compact ~ /^"[^"]*"$/) {
                    out_class = "string"
                    out_value = substr(compact, 2, length(compact) - 2)
                    return
                }
                if (normalize_number(compact) ~ /^[-+]?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/) {
                    out_class = "number"
                    out_value = normalize_scalar(compact)
                    return
                }
                if (compact ~ /::/) {
                    n = split(compact, parts, /::/)
                    token = parts[n]
                    out_class = "enum"
                    out_value = normalize_token(token)
                    return
                }
                out_class = "symbol"
                out_value = normalize_token(compact)
            }
            function print_default(id, expr) {
                if (id == "") {
                    return
                }
                id = normalize_option_id(id)
                classify_expr(expr)
                print id "\t" out_class "\t" out_value "\t" out_raw
            }
            function flush_static(    buf, args, arg1, arg2, id) {
                buf = static_buf
                if (buf ~ /Property::with_default[[:space:]]*\(/) {
                    args = extract_call_args(buf, "Property::with_default")
                    split_next_arg(args)
                    arg1 = parsed_arg
                    split_next_arg(parsed_rest)
                    arg2 = parsed_arg
                    id = read_quoted(arg1)
                    print_default(id, arg2)
                } else if (buf ~ /Property::new[[:space:]]*\(/) {
                    args = extract_call_args(buf, "Property::new")
                    split_next_arg(args)
                    arg1 = parsed_arg
                    id = read_quoted(arg1)
                    print_default(id, "None")
                }

                in_static = 0
                static_name_pending = ""
                static_buf = ""
                static_paren_depth = 0
                static_brace_depth = 0
            }
            {
                line = $0

                if (in_static == 1) {
                    static_buf = static_buf " " line
                    update_static_depth(line)
                    if (line ~ /;/ && static_paren_depth <= 0 && static_brace_depth <= 0) {
                        flush_static()
                    }
                }

                if (in_static == 0 && line ~ /pub static [A-Z0-9_]+[[:space:]]*:/) {
                    static_name_pending = line
                    sub(/^.*pub static[[:space:]]+/, "", static_name_pending)
                    sub(/[[:space:]]*:.*$/, "", static_name_pending)
                    static_name_pending = trim_ws(static_name_pending)
                    static_buf = line
                    static_paren_depth = 0
                    static_brace_depth = 0
                    update_static_depth(line)
                    in_static = 1
                    if (line ~ /;/ && static_paren_depth <= 0 && static_brace_depth <= 0) {
                        flush_static()
                    }
                }
            }
        ' "$file"
    done | sort -u > "$rust_option_defaults_raw_file"

awk -F '\t' '
    function score(cls) {
        if (cls == "number" || cls == "bool" || cls == "string" || cls == "enum" || cls == "list" || cls == "object") {
            return 4
        }
        if (cls == "symbol") {
            return 3
        }
        if (cls == "null") {
            return 2
        }
        return 1
    }
    {
        id = $1
        cls = $2
        if (!(id in best_score) || score(cls) > best_score[id]) {
            best_score[id] = score(cls)
            best_line[id] = $0
        }
    }
    END {
        for (id in best_line) {
            print best_line[id]
        }
    }
' "$rust_option_defaults_raw_file" | sort -u > "$rust_option_defaults_file"

# Restrict to option ids actually used by Java algorithm option-support registrations.
awk -F '\t' '
    FNR == NR {
        keep[$1] = 1
        next
    }
    ($1 in keep)
' "$java_supported_option_ids_file" "$java_option_defaults_file" > "$java_filtered_file"

awk -F '\t' '
    FNR == NR {
        keep[$1] = 1
        next
    }
    ($1 in keep)
' "$java_supported_option_ids_file" "$rust_option_defaults_file" > "$rust_filtered_file"

cut -f1 "$java_filtered_file" | sort -u > "$java_ids_file"
cut -f1 "$rust_filtered_file" | sort -u > "$rust_ids_file"

comm -23 "$java_ids_file" "$rust_ids_file" > "$missing_ids_file"
comm -13 "$java_ids_file" "$rust_ids_file" > "$extra_ids_file"

: > "$uncomparable_file"
awk -F '\t' -v uncomparable_file="$uncomparable_file" '
    function is_comparable(cls) {
        return (cls == "null" || cls == "bool" || cls == "number" || cls == "string" || cls == "enum" || cls == "list" || cls == "object")
    }
    FNR == NR {
        j_class[$1] = $2
        j_value[$1] = $3
        j_raw[$1] = $4
        next
    }
    {
        id = $1
        if (!(id in j_class)) {
            next
        }
        r_class = $2
        r_value = $3
        r_raw = $4
        if (is_comparable(j_class[id]) && is_comparable(r_class)) {
            if (j_class[id] != r_class || j_value[id] != r_value) {
                print id "\t" j_class[id] "\t" j_value[id] "\t" r_class "\t" r_value "\t" j_raw[id] "\t" r_raw
            }
        } else {
            print id "\t" j_class[id] "\t" j_value[id] "\t" r_class "\t" r_value "\t" j_raw[id] "\t" r_raw > uncomparable_file
        }
    }
' "$java_filtered_file" "$rust_filtered_file" > "$value_mismatches_file"

missing_count="$(wc -l < "$missing_ids_file" | tr -d ' ')"
extra_count="$(wc -l < "$extra_ids_file" | tr -d ' ')"
mismatch_count="$(wc -l < "$value_mismatches_file" | tr -d ' ')"
uncomparable_count="$(wc -l < "$uncomparable_file" | tr -d ' ')"
java_total="$(wc -l < "$java_ids_file" | tr -d ' ')"
rust_total="$(wc -l < "$rust_ids_file" | tr -d ' ')"

status="ok"
if [ "$missing_count" -gt 0 ] || [ "$extra_count" -gt 0 ] || [ "$mismatch_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm Option Default Value Parity"
    echo
    echo "- status: $status"
    echo "- java option ids considered: $java_total"
    echo "- rust option ids considered: $rust_total"
    echo "- missing option ids in rust: $missing_count"
    echo "- extra option ids in rust: $extra_count"
    echo "- comparable value mismatches: $mismatch_count"
    echo "- uncomparable pairs (informational): $uncomparable_count"
    echo
    echo "## Missing Option IDs In Rust"
    if [ "$missing_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$missing_ids_file"
    fi
    echo
    echo "## Extra Option IDs In Rust"
    if [ "$extra_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$extra_ids_file"
    fi
    echo
    echo "## Comparable Value Mismatches (option | java_class | java_value | rust_class | rust_value | java_raw | rust_raw)"
    if [ "$mismatch_count" -eq 0 ]; then
        echo "- none"
    else
        while IFS="$(printf '\t')" read -r id jc jv rc rv jraw rraw; do
            printf -- "- %s | %s | %s | %s | %s | %s | %s\n" "$id" "$jc" "$jv" "$rc" "$rv" "$jraw" "$rraw"
        done < "$value_mismatches_file"
    fi
    echo
    echo "## Uncomparable Pairs (informational, first 50)"
    if [ "$uncomparable_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' 'NR <= 50 { printf("- %s | %s | %s | %s | %s | %s | %s\n", $1, $2, $3, $4, $5, $6, $7) }' "$uncomparable_file"
    fi
} > "$REPORT_FILE"

if [ "$STRICT_MODE" = "true" ] && [ "$status" != "ok" ]; then
    echo "algorithm option default value parity drift detected (strict mode): $REPORT_FILE" >&2
    exit 1
fi

echo "wrote algorithm option default value parity report: $REPORT_FILE"
