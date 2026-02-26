#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
RUST_CORE_DATA_FILE="${RUST_CORE_DATA_FILE:-$RUST_SOURCES_ROOT/org.eclipse.elk.core/src/org/eclipse/elk/core/data/mod.rs}"
REPORT_FILE="${1:-parity/algorithm_option_default_parity.md}"
STRICT_MODE="${ALGORITHM_OPTION_DEFAULT_PARITY_STRICT:-false}"
KEEP_TMP="${ALGORITHM_OPTION_DEFAULT_PARITY_KEEP_TMP:-false}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-option-default-parity.XXXXXX")"
cleanup_tmp() {
    if [ "$KEEP_TMP" = "true" ]; then
        echo "keeping temp directory: $tmp_dir" >&2
        return
    fi
    rm -rf "$tmp_dir"
}
trap cleanup_tmp EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

rust_algo_map_file="$tmp_dir/rust_algo_map.tsv"
rust_option_symbol_raw_file="$tmp_dir/rust_option_symbol_raw.tsv"
rust_option_symbol_map_file="$tmp_dir/rust_option_symbol_map.tsv"

java_option_default_kind_raw_file="$tmp_dir/java_option_default_kind.raw.tsv"
java_option_default_kind_file="$tmp_dir/java_option_default_kind.tsv"

java_defaults_file="$tmp_dir/java_defaults.tsv"
rust_provider_defaults_file="$tmp_dir/rust_provider_defaults.tsv"
rust_core_defaults_file="$tmp_dir/rust_core_defaults.tsv"
rust_core_defaults_filtered_file="$tmp_dir/rust_core_defaults.filtered.tsv"
rust_provider_algorithms_file="$tmp_dir/rust_provider_algorithms.txt"
rust_defaults_file="$tmp_dir/rust_defaults.tsv"

java_counts_file="$tmp_dir/java_counts.tsv"
rust_counts_file="$tmp_dir/rust_counts.tsv"
combined_counts_file="$tmp_dir/combined_counts.tsv"

java_keys_file="$tmp_dir/java_keys.tsv"
rust_keys_file="$tmp_dir/rust_keys.tsv"
missing_pairs_file="$tmp_dir/missing_pairs.tsv"
extra_pairs_file="$tmp_dir/extra_pairs.tsv"
pair_status_file="$tmp_dir/pair_status.tsv"
explicit_mismatch_file="$tmp_dir/explicit_mismatch.tsv"
java_unknown_mode_file="$tmp_dir/java_unknown_mode.tsv"

# Build struct-name -> ALGORITHM_ID map from Rust option files.
(rg --files "$RUST_SOURCES_ROOT" -g '*options.rs' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            /pub struct [A-Za-z_][A-Za-z0-9_]*/ {
                name = $0
                sub(/^.*pub struct[[:space:]]+/, "", name)
                sub(/[^A-Za-z0-9_].*$/, "", name)
                current = name
                next
            }
            /pub const ALGORITHM_ID[[:space:]]*:[^=]*=[[:space:]]*"org\.eclipse\.elk/ {
                if (current == "") {
                    next
                }
                if (!match($0, /"org\.eclipse\.elk[^"]+"/)) {
                    next
                }
                id = substr($0, RSTART + 1, RLENGTH - 2)
                print current "\t" id
            }
        ' "$file"
    done | sort -u > "$rust_algo_map_file"

# Build Rust option-symbol -> option-id map from *options.rs.
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
            function flush_static(    compact, id) {
                if (static_name_pending == "") {
                    in_static = 0
                    static_buf = ""
                    static_paren_depth = 0
                    static_brace_depth = 0
                    return
                }

                compact = remove_ws(static_buf)
                if (compact ~ /Property</ && match(static_buf, /"org\.eclipse\.elk[^"]+"/)) {
                    id = substr(static_buf, RSTART + 1, RLENGTH - 2)
                    property_id[static_name_pending] = id
                }

                in_static = 0
                static_name_pending = ""
                static_buf = ""
                static_paren_depth = 0
                static_brace_depth = 0
            }
            function flush_const(    rhs, symbol) {
                rhs = const_buf
                if (rhs !~ /=/) {
                    in_const = 0
                    const_name_pending = ""
                    const_buf = ""
                    return
                }
                sub(/^.*=[[:space:]]*/, "", rhs)
                sub(/[[:space:]]*;[[:space:]]*$/, "", rhs)
                rhs = trim_ws(rhs)
                if (rhs ~ /^&/) {
                    rhs = substr(rhs, 2)
                }
                rhs = remove_ws(rhs)

                symbol = current_struct "::" const_name_pending
                if (rhs in property_id) {
                    print symbol "\t" property_id[rhs]
                } else if (rhs ~ /^[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*$/) {
                    print symbol "\t@alias:" rhs
                }

                in_const = 0
                const_name_pending = ""
                const_buf = ""
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

                if (line ~ /impl [A-Za-z_][A-Za-z0-9_]*[[:space:]]*{/) {
                    current_struct = line
                    sub(/^.*impl[[:space:]]+/, "", current_struct)
                    sub(/[[:space:]]*{.*$/, "", current_struct)
                    current_struct = trim_ws(current_struct)
                } else if (line ~ /^[[:space:]]*}/) {
                    current_struct = ""
                }

                if (in_const == 1) {
                    const_buf = const_buf " " line
                    if (line ~ /;/) {
                        flush_const()
                    }
                    next
                }

                if (current_struct != "" && line ~ /pub const [A-Z0-9_]+/) {
                    const_name = line
                    sub(/^.*pub const[[:space:]]+/, "", const_name)
                    sub(/[[:space:]]*:.*$/, "", const_name)
                    const_name = trim_ws(const_name)

                    const_name_pending = const_name
                    const_buf = line
                    if (line ~ /;/) {
                        flush_const()
                    } else {
                        in_const = 1
                    }
                    next
                }
            }
        ' "$file"
    done | sort -u > "$rust_option_symbol_raw_file"

awk -F '\t' '
    function resolve(key, depth,    target, out) {
        if (depth > 40) {
            return ""
        }
        if (key in direct) {
            return direct[key]
        }
        if (!(key in alias)) {
            return ""
        }
        target = alias[key]
        out = resolve(target, depth + 1)
        return out
    }
    {
        key = $1
        value = $2
        all[key] = 1
        if (value ~ /^@alias:/) {
            sub(/^@alias:/, "", value)
            alias[key] = value
            all[value] = 1
        } else if (value != "") {
            direct[key] = value
        }
    }
    END {
        for (key in all) {
            out = resolve(key, 0)
            if (out != "") {
                print key "\t" out
            }
        }
    }
' "$rust_option_symbol_raw_file" | sort -u > "$rust_option_symbol_map_file"

# Java option-id -> default kind map (null/nonnull/unknown) from LayoutOptionData.Builder blocks.
(rg --files "$JAVA_SOURCES_ROOT" -g '*.java' || true) \
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
            function read_option_id(text,    value) {
                if (!match(text, /"org\.eclipse\.elk[^"]+"/)) {
                    return ""
                }
                value = substr(text, RSTART + 1, RLENGTH - 2)
                return value
            }
            function classify(expr, compact) {
                compact = remove_ws(expr)
                if (compact == "") {
                    return "unknown"
                }
                if (compact == "null") {
                    return "null"
                }
                if (compact in const_kind) {
                    return const_kind[compact]
                }
                return "nonnull"
            }
            function flush_option() {
                if (option_id != "") {
                    print option_id "\t" option_kind
                }
                in_option = 0
                option_id = ""
                option_kind = "null"
            }

            /[[:space:]]static final [^=]*[A-Z0-9_]+[[:space:]]*=/ {
                name = $0
                sub(/^.*[[:space:]]+/, "", name)
                sub(/[[:space:]]*=.*$/, "", name)
                name = trim_ws(name)
                if (name !~ /^[A-Z0-9_]+$/) {
                    next
                }

                rhs = $0
                sub(/^.*=[[:space:]]*/, "", rhs)
                sub(/[[:space:]]*;[[:space:]]*$/, "", rhs)
                rhs = remove_ws(rhs)
                if (rhs == "null") {
                    const_kind[name] = "null"
                } else if (rhs != "") {
                    const_kind[name] = "nonnull"
                }
            }

            /LayoutOptionData\.Builder\(\)/ {
                if (in_option == 1) {
                    flush_option()
                }
                in_option = 1
                option_id = ""
                option_kind = "null"
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
                expr = trim_ws(expr)
                option_kind = classify(expr)
                next
            }

            in_option == 1 && /\.create\(\)/ {
                flush_option()
                next
            }

            END {
                if (in_option == 1) {
                    flush_option()
                }
            }
        ' "$file"
    done | sort > "$java_option_default_kind_raw_file"

awk -F '\t' '
    {
        id = $1
        kind = $2
        if (id == "" || kind == "") {
            next
        }
        if (!(id in best)) {
            best[id] = kind
            next
        }
        if (kind == "nonnull") {
            best[id] = "nonnull"
        } else if (kind == "null" && best[id] != "nonnull") {
            best[id] = "null"
        }
    }
    END {
        for (id in best) {
            print id "\t" best[id]
        }
    }
' "$java_option_default_kind_raw_file" | sort -u > "$java_option_default_kind_file"

# Java defaults: algorithm + option-id + default mode from addOptionSupport calls.
(rg --files "$JAVA_SOURCES_ROOT" -g '*Options.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk -v option_kind_file="$java_option_default_kind_file" '
            BEGIN {
                while ((getline line < option_kind_file) > 0) {
                    split(line, parts, "\t")
                    if (parts[1] != "" && parts[2] != "") {
                        option_kind_by_id[parts[1]] = parts[2]
                    }
                }
                close(option_kind_file)
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
            function classify(expr, option_id, compact, mapped_kind) {
                compact = remove_ws(expr)
                if (compact == "null") {
                    return "explicit_null"
                }
                if (compact in const_kind) {
                    if (const_kind[compact] == "null") {
                        return "explicit_null"
                    }
                    return "explicit_nonnull"
                }
                if (compact ~ /getDefault\(\)$/) {
                    if (option_id in option_kind_by_id) {
                        mapped_kind = option_kind_by_id[option_id]
                        if (mapped_kind == "null") {
                            return "getdefault_null"
                        }
                        if (mapped_kind == "nonnull") {
                            return "getdefault_nonnull"
                        }
                    }
                    return "getdefault_unknown"
                }
                return "explicit_nonnull"
            }
            function flush_call(text,    call, first, second, rest, algo, expr, kind) {
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
                rest = substr(rest, RSTART + RLENGTH)

                sub(/^[[:space:]]*,[[:space:]]*/, "", rest)
                sub(/\)[[:space:]]*;[[:space:]]*$/, "", rest)
                expr = trim_ws(rest)
                if (expr == "") {
                    return
                }
                kind = classify(expr, second)
                algo = first
                print algo "\t" second "\t" kind
            }

            /public static final [^=]*[A-Z0-9_]+_DEFAULT[[:space:]]*=/ {
                name = $0
                sub(/^.*public static final[[:space:]]+[^[:space:]]+[[:space:]]+/, "", name)
                sub(/[[:space:]]*=.*$/, "", name)
                name = trim_ws(name)

                rhs = $0
                sub(/^.*=[[:space:]]*/, "", rhs)
                sub(/[[:space:]]*;[[:space:]]*$/, "", rhs)
                rhs = remove_ws(rhs)
                if (rhs == "null") {
                    const_kind[name] = "null"
                } else if (rhs != "") {
                    const_kind[name] = "nonnull"
                }
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
    done | sort > "$java_defaults_file"

# Rust provider defaults: algorithm + option-id + default mode from add_option_support calls.
(rg --files "$RUST_SOURCES_ROOT" -g '*meta_data_provider.rs' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk -v algo_map_file="$rust_algo_map_file" -v option_map_file="$rust_option_symbol_map_file" '
            BEGIN {
                while ((getline line < algo_map_file) > 0) {
                    split(line, parts, "\t")
                    if (parts[1] != "" && parts[2] != "") {
                        algo_id_by_struct[parts[1]] = parts[2]
                    }
                }
                close(algo_map_file)

                while ((getline line < option_map_file) > 0) {
                    split(line, parts, "\t")
                    if (parts[1] != "" && parts[2] != "") {
                        option_id_by_symbol[parts[1]] = parts[2]
                    }
                }
                close(option_map_file)
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
            function split_next_arg(text,    depth, i, ch) {
                depth = 0
                for (i = 1; i <= length(text); i++) {
                    ch = substr(text, i, 1)
                    if (ch == "(") {
                        depth++
                    } else if (ch == ")") {
                        if (depth > 0) {
                            depth--
                        }
                    } else if (ch == "," && depth == 0) {
                        parsed_arg = trim_ws(substr(text, 1, i - 1))
                        parsed_rest = trim_ws(substr(text, i + 1))
                        return
                    }
                }
                    parsed_arg = trim_ws(text)
                    parsed_rest = ""
            }
            function resolve_algo(token,    value, n, parts, key) {
                value = remove_ws(token)
                if (value ~ /^"org\.eclipse\.elk/) {
                    gsub(/"/, "", value)
                    return value
                }
                if (value ~ /::ALGORITHM_ID$/) {
                    sub(/::ALGORITHM_ID$/, "", value)
                    n = split(value, parts, /::/)
                    key = parts[n]
                    return algo_id_by_struct[key]
                }
                if (value in local_var_expr) {
                    return resolve_algo(local_var_expr[value])
                }
                return ""
            }
            function normalize_option_id(id) {
                if (id ~ /^org\.eclipse\.elk\.alg\.layered\./) {
                    sub(/^org\.eclipse\.elk\.alg\.layered\./, "org.eclipse.elk.layered.", id)
                }
                return id
            }
            function resolve_option(token,    value, n, parts, key, out) {
                value = remove_ws(token)
                if (value == "") {
                    return ""
                }
                if (value ~ /^"org\.eclipse\.elk/) {
                    gsub(/"/, "", value)
                    return normalize_option_id(value)
                }
                if (value in local_var_expr) {
                    return resolve_option(local_var_expr[value])
                }
                if (value ~ /^&/) {
                    value = substr(value, 2)
                }
                if (value ~ /\.id\(\)$/) {
                    sub(/\.id\(\)$/, "", value)
                }
                if (value in option_id_by_symbol) {
                    out = option_id_by_symbol[value]
                    return normalize_option_id(out)
                }
                if (value ~ /^[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*$/) {
                    out = option_id_by_symbol[value]
                    return normalize_option_id(out)
                }
                return ""
            }
            function classify_default(expr, value) {
                value = remove_ws(expr)
                if (value == "None") {
                    return "none"
                }
                return "nonnull"
            }
            function flush_call(text,    call, a1, a2, a3, algo, option, kind) {
                call = text
                sub(/^.*registry\.add_option_support[[:space:]]*\(/, "", call)
                sub(/\)[[:space:]]*;[[:space:]]*$/, "", call)

                split_next_arg(call)
                a1 = parsed_arg
                call = parsed_rest
                split_next_arg(call)
                a2 = parsed_arg
                a3 = parsed_rest

                algo = resolve_algo(a1)
                if (algo == "") {
                    return
                }
                option = resolve_option(a2)
                if (option == "") {
                    return
                }
                kind = classify_default(a3)
                print algo "\t" option "\t" kind
            }

            /let[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[[:space:]]*[^;]+;[[:space:]]*$/ {
                var_name = $0
                sub(/^.*let[[:space:]]+/, "", var_name)
                sub(/[[:space:]]*=.*$/, "", var_name)
                var_name = trim_ws(var_name)

                expr = $0
                sub(/^.*=[[:space:]]*/, "", expr)
                sub(/[[:space:]]*;[[:space:]]*$/, "", expr)
                local_var_expr[var_name] = trim_ws(expr)
            }

            {
                if (!in_call && $0 ~ /registry\.add_option_support[[:space:]]*\(/) {
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
    done | sort > "$rust_provider_defaults_file"

# Rust core defaults: register_core_algorithms() add_known_option_default calls.
if [ -f "$RUST_CORE_DATA_FILE" ]; then
    awk -v algo_map_file="$rust_algo_map_file" -v option_map_file="$rust_option_symbol_map_file" '
        BEGIN {
            while ((getline line < algo_map_file) > 0) {
                split(line, parts, "\t")
                if (parts[1] != "" && parts[2] != "") {
                    algo_id_by_struct[parts[1]] = parts[2]
                }
            }
            close(algo_map_file)

            while ((getline line < option_map_file) > 0) {
                split(line, parts, "\t")
                if (parts[1] != "" && parts[2] != "") {
                    option_id_by_symbol[parts[1]] = parts[2]
                }
            }
            close(option_map_file)
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
        function split_next_arg(text,    depth, i, ch) {
            depth = 0
            for (i = 1; i <= length(text); i++) {
                ch = substr(text, i, 1)
                if (ch == "(") {
                    depth++
                } else if (ch == ")") {
                    if (depth > 0) {
                        depth--
                    }
                } else if (ch == "," && depth == 0) {
                    parsed_arg = trim_ws(substr(text, 1, i - 1))
                    parsed_rest = trim_ws(substr(text, i + 1))
                    return
                }
            }
            parsed_arg = trim_ws(text)
            parsed_rest = ""
        }
        function resolve_algo(token,    value, n, parts, key) {
            value = remove_ws(token)
            if (value ~ /^"org\.eclipse\.elk/) {
                gsub(/"/, "", value)
                return value
            }
            if (value ~ /::ALGORITHM_ID$/) {
                sub(/::ALGORITHM_ID$/, "", value)
                n = split(value, parts, /::/)
                key = parts[n]
                return algo_id_by_struct[key]
            }
            return ""
        }
        function normalize_option_id(id) {
            if (id ~ /^org\.eclipse\.elk\.alg\.layered\./) {
                sub(/^org\.eclipse\.elk\.alg\.layered\./, "org.eclipse.elk.layered.", id)
            }
            return id
        }
        function resolve_option(token,    value, out) {
            value = remove_ws(token)
            if (value == "") {
                return ""
            }
            if (value ~ /^"org\.eclipse\.elk/) {
                gsub(/"/, "", value)
                return normalize_option_id(value)
            }
            if (value ~ /^&/) {
                value = substr(value, 2)
            }
            if (value ~ /\.id\(\)$/) {
                sub(/\.id\(\)$/, "", value)
            }
            if (value in option_id_by_symbol) {
                out = option_id_by_symbol[value]
                return normalize_option_id(out)
            }
            if (value ~ /^[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*$/) {
                out = option_id_by_symbol[value]
                return normalize_option_id(out)
            }
            return ""
        }
        function classify_default(expr, value) {
            value = remove_ws(expr)
            if (value == "None") {
                return "none"
            }
            return "nonnull"
        }
        function flush_call(text,    call, var_name, a1, a2, algo, option, kind) {
            call = text
            var_name = call
            sub(/^[[:space:]]*/, "", var_name)
            sub(/\.add_known_option_default[[:space:]]*\(.*/, "", var_name)
            var_name = trim_ws(var_name)
            if (!(var_name in algorithm_by_var)) {
                return
            }
            algo = algorithm_by_var[var_name]
            sub(/^.*\.add_known_option_default[[:space:]]*\(/, "", call)
            sub(/\)[[:space:]]*;[[:space:]]*$/, "", call)

            split_next_arg(call)
            a1 = parsed_arg
            a2 = parsed_rest

            option = resolve_option(a1)
            if (option == "") {
                return
            }
            kind = classify_default(a2)
            print algo "\t" option "\t" kind
        }

        /let[[:space:]]+mut[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[[:space:]]*LayoutAlgorithmData::new\(/ {
            var_name = $0
            sub(/^.*let[[:space:]]+mut[[:space:]]+/, "", var_name)
            sub(/[[:space:]]*=.*$/, "", var_name)
            var_name = trim_ws(var_name)

            algo_expr = $0
            sub(/^.*LayoutAlgorithmData::new\(/, "", algo_expr)
            sub(/\).*/, "", algo_expr)
            algo_expr = trim_ws(algo_expr)
            algo_id = resolve_algo(algo_expr)
            if (algo_id != "") {
                algorithm_by_var[var_name] = algo_id
            }
        }
        {
            if (!in_call && $0 ~ /[A-Za-z_][A-Za-z0-9_]*\.add_known_option_default[[:space:]]*\(/) {
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
    ' "$RUST_CORE_DATA_FILE" | sort > "$rust_core_defaults_file"
else
    : > "$rust_core_defaults_file"
fi

cut -f1 "$rust_provider_defaults_file" | sort -u > "$rust_provider_algorithms_file"
awk -F '\t' '
    FNR == NR {
        provider[$1] = 1
        next
    }
    !($1 in provider)
' "$rust_provider_algorithms_file" "$rust_core_defaults_file" > "$rust_core_defaults_filtered_file"

cat "$rust_provider_defaults_file" "$rust_core_defaults_filtered_file" | sort > "$rust_defaults_file"

awk -F '\t' '
    {
        algo = $1
        kind = $3
        total[algo] += 1
        if (kind == "explicit_null") {
            explicit_null[algo] += 1
        } else if (kind == "explicit_nonnull") {
            explicit_nonnull[algo] += 1
        } else if (kind == "getdefault_null") {
            getdefault_null[algo] += 1
        } else if (kind == "getdefault_nonnull") {
            getdefault_nonnull[algo] += 1
        } else if (kind == "getdefault_unknown") {
            getdefault_unknown[algo] += 1
        } else {
            unknowns[algo] += 1
        }
    }
    END {
        for (algo in total) {
            printf "%s\t%d\t%d\t%d\t%d\t%d\t%d\t%d\n", \
                algo, total[algo], \
                explicit_null[algo] + 0, explicit_nonnull[algo] + 0, \
                getdefault_null[algo] + 0, getdefault_nonnull[algo] + 0, \
                getdefault_unknown[algo] + 0, unknowns[algo] + 0
        }
    }
' "$java_defaults_file" | sort > "$java_counts_file"

awk -F '\t' '
    {
        algo = $1
        kind = $3
        total[algo] += 1
        if (kind == "none") {
            none[algo] += 1
        } else if (kind == "nonnull") {
            nonnulls[algo] += 1
        }
    }
    END {
        for (algo in total) {
            printf "%s\t%d\t%d\t%d\n", algo, total[algo], none[algo] + 0, nonnulls[algo] + 0
        }
    }
' "$rust_defaults_file" | sort > "$rust_counts_file"

awk -F '\t' '
    FNR == NR {
        j_total[$1] = $2
        j_explicit_null[$1] = $3
        j_explicit_nonnull[$1] = $4
        j_getdefault_null[$1] = $5
        j_getdefault_nonnull[$1] = $6
        j_getdefault_unknown[$1] = $7
        j_unknown[$1] = $8
        all[$1] = 1
        next
    }
    {
        r_total[$1] = $2
        r_none[$1] = $3
        r_nonnull[$1] = $4
        all[$1] = 1
    }
    END {
        for (algo in all) {
            jt = (algo in j_total) ? j_total[algo] : 0
            je0 = (algo in j_explicit_null) ? j_explicit_null[algo] : 0
            je1 = (algo in j_explicit_nonnull) ? j_explicit_nonnull[algo] : 0
            jg0 = (algo in j_getdefault_null) ? j_getdefault_null[algo] : 0
            jg1 = (algo in j_getdefault_nonnull) ? j_getdefault_nonnull[algo] : 0
            jgu = (algo in j_getdefault_unknown) ? j_getdefault_unknown[algo] : 0
            ju = (algo in j_unknown) ? j_unknown[algo] : 0
            rt = (algo in r_total) ? r_total[algo] : 0
            rn = (algo in r_none) ? r_none[algo] : 0
            rx = (algo in r_nonnull) ? r_nonnull[algo] : 0
            printf "%s\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\n", \
                algo, jt, je0, je1, jg0, jg1, jgu, ju, rt, rn, rx, rt - jt
        }
    }
' "$java_counts_file" "$rust_counts_file" | sort > "$combined_counts_file"

cut -f1,2 "$java_defaults_file" | sort -u > "$java_keys_file"
cut -f1,2 "$rust_defaults_file" | sort -u > "$rust_keys_file"

comm -23 "$java_keys_file" "$rust_keys_file" > "$missing_pairs_file"
comm -13 "$java_keys_file" "$rust_keys_file" > "$extra_pairs_file"

awk -F '\t' '
    FNR == NR {
        key = $1 "\t" $2
        java_mode[key] = $3
        next
    }
    {
        key = $1 "\t" $2
        rust_mode[key] = $3
        all[key] = 1
    }
    END {
        for (key in java_mode) {
            all[key] = 1
        }
        for (key in all) {
            split(key, parts, "\t")
            algo = parts[1]
            option = parts[2]
            jm = (key in java_mode) ? java_mode[key] : ""
            rm = (key in rust_mode) ? rust_mode[key] : ""
            status = "ok"

            if (jm == "") {
                status = "extra_in_rust"
            } else if (rm == "") {
                status = "missing_in_rust"
            } else if (jm == "explicit_nonnull" && rm != "nonnull") {
                status = "mismatch_explicit_nonnull"
            } else if (jm == "explicit_null" && rm != "none") {
                status = "mismatch_explicit_null"
            } else if (jm == "unknown") {
                status = "unknown_java_mode"
            } else if (jm ~ /^getdefault_/) {
                status = "ok_getdefault"
            }

            print algo "\t" option "\t" jm "\t" rm "\t" status
        }
    }
' "$java_defaults_file" "$rust_defaults_file" | sort > "$pair_status_file"

awk -F '\t' '$5 ~ /^mismatch_explicit_/' "$pair_status_file" > "$explicit_mismatch_file"
awk -F '\t' '$5 == "unknown_java_mode"' "$pair_status_file" > "$java_unknown_mode_file"

algorithm_count="$(wc -l < "$combined_counts_file" | tr -d ' ')"
pair_count="$(wc -l < "$pair_status_file" | tr -d ' ')"
missing_pair_count="$(wc -l < "$missing_pairs_file" | tr -d ' ')"
extra_pair_count="$(wc -l < "$extra_pairs_file" | tr -d ' ')"
explicit_mismatch_count="$(wc -l < "$explicit_mismatch_file" | tr -d ' ')"
java_unknown_mode_count="$(wc -l < "$java_unknown_mode_file" | tr -d ' ')"
getdefault_pair_count="$(awk -F '\t' '$3 ~ /^getdefault_/{c++} END{print c+0}' "$pair_status_file")"

status="ok"
if [ "$explicit_mismatch_count" -gt 0 ] || [ "$java_unknown_mode_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm Option Default Parity"
    echo
    echo "- status: $status"
    echo "- algorithms compared: $algorithm_count"
    echo "- option pairs compared: $pair_count"
    echo "- missing option pairs in rust (informational): $missing_pair_count"
    echo "- extra option pairs in rust (informational): $extra_pair_count"
    echo "- explicit default mismatches (java explicit_* vs rust mode): $explicit_mismatch_count"
    echo "- java unknown modes: $java_unknown_mode_count"
    echo "- java getDefault pairs observed: $getdefault_pair_count"
    echo
    echo "## Per-Algorithm Counts (algo | java_total | java_explicit_null | java_explicit_nonnull | java_getdefault_null | java_getdefault_nonnull | java_getdefault_unknown | java_unknown | rust_total | rust_none | rust_nonnull | total_delta)"
    if [ -s "$combined_counts_file" ]; then
        while IFS="$(printf '\t')" read -r algo j_total je0 je1 jg0 jg1 jgu ju r_total rn rx total_delta; do
            printf -- "- %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s\n" \
                "$algo" "$j_total" "$je0" "$je1" "$jg0" "$jg1" "$jgu" "$ju" "$r_total" "$rn" "$rx" "$total_delta"
        done < "$combined_counts_file"
    else
        echo "- none"
    fi
    echo
    echo "## Missing Option Pairs In Rust (algo | option)"
    if [ -s "$missing_pairs_file" ]; then
        while IFS="$(printf '\t')" read -r algo option; do
            printf -- "- %s | %s\n" "$algo" "$option"
        done < "$missing_pairs_file"
    else
        echo "- none"
    fi
    echo
    echo "## Extra Option Pairs In Rust (algo | option)"
    if [ -s "$extra_pairs_file" ]; then
        while IFS="$(printf '\t')" read -r algo option; do
            printf -- "- %s | %s\n" "$algo" "$option"
        done < "$extra_pairs_file"
    else
        echo "- none"
    fi
    echo
    echo "## Explicit Default Mismatches (algo | option | java_mode | rust_mode)"
    if [ -s "$explicit_mismatch_file" ]; then
        while IFS="$(printf '\t')" read -r algo option java_mode rust_mode _; do
            printf -- "- %s | %s | %s | %s\n" "$algo" "$option" "$java_mode" "$rust_mode"
        done < "$explicit_mismatch_file"
    else
        echo "- none"
    fi
    echo
    echo "## Java Unknown Modes (algo | option | java_mode | rust_mode)"
    if [ -s "$java_unknown_mode_file" ]; then
        while IFS="$(printf '\t')" read -r algo option java_mode rust_mode _; do
            printf -- "- %s | %s | %s | %s\n" "$algo" "$option" "$java_mode" "$rust_mode"
        done < "$java_unknown_mode_file"
    else
        echo "- none"
    fi
} > "$REPORT_FILE"

if [ "$STRICT_MODE" = "true" ] && [ "$status" != "ok" ]; then
    echo "algorithm option default parity drift detected (strict mode): $REPORT_FILE" >&2
    exit 1
fi

echo "wrote algorithm option default parity report: $REPORT_FILE"
