#!/usr/bin/env sh
set -eu

JAVA_CORE_OPTIONS="${JAVA_CORE_OPTIONS:-external/elk/plugins/org.eclipse.elk.core/src-gen/org/eclipse/elk/core/options/CoreOptions.java}"
RUST_CORE_OPTIONS="${RUST_CORE_OPTIONS:-plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/options/core_options.rs}"
RUST_CORE_META="${RUST_CORE_META:-plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/options/core_options_meta.rs}"
REPORT_FILE="${1:-tests/core_option_dependency_parity.md}"
STRICT_MODE="${CORE_OPTION_DEPENDENCY_PARITY_STRICT:-true}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/core-option-dependency-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

rust_option_map_file="$tmp_dir/rust_option_map.tsv"
java_dependency_constants_file="$tmp_dir/java_dependency_constants.tsv"
java_dependencies_file="$tmp_dir/java_dependencies.tsv"
rust_dependencies_file="$tmp_dir/rust_dependencies.tsv"

java_only_file="$tmp_dir/java_only.tsv"
rust_only_file="$tmp_dir/rust_only.tsv"
value_mismatch_file="$tmp_dir/value_mismatch.tsv"

awk '
    function trim_ws(text) {
        gsub(/^[[:space:]]+/, "", text)
        gsub(/[[:space:]]+$/, "", text)
        return text
    }
    /pub static [A-Z0-9_]+:[[:space:]]*LazyLock<Property/ {
        symbol = $0
        sub(/^.*pub static[[:space:]]+/, "", symbol)
        sub(/:.*/, "", symbol)
        current_property = trim_ws(symbol)
    }
    current_property != "" && match($0, /"org\.eclipse\.elk[^"]+"/) {
        id = substr($0, RSTART + 1, RLENGTH - 2)
        property_id_by_symbol[current_property] = id
        current_property = ""
    }
    /pub const [A-Z0-9_]+:[[:space:]]*.*LazyLock<Property/ {
        alias = $0
        sub(/^.*pub const[[:space:]]+/, "", alias)
        sub(/:.*/, "", alias)
        current_alias = trim_ws(alias)
    }
    current_alias != "" && /&[A-Z0-9_]+;/ {
        property_symbol = $0
        sub(/^.*&/, "", property_symbol)
        sub(/;.*/, "", property_symbol)
        alias_to_property[current_alias] = trim_ws(property_symbol)
        current_alias = ""
    }
    END {
        for (symbol in property_id_by_symbol) {
            print symbol "\t" property_id_by_symbol[symbol]
        }
        for (alias in alias_to_property) {
            symbol = alias_to_property[alias]
            if (symbol in property_id_by_symbol) {
                print alias "\t" property_id_by_symbol[symbol]
            }
        }
    }
' "$RUST_CORE_OPTIONS" | sort -u > "$rust_option_map_file"

awk '
    function trim_ws(text) {
        gsub(/^[[:space:]]+/, "", text)
        gsub(/[[:space:]]+$/, "", text)
        return text
    }
    function pascal_from_upper(text,    n, parts, i, part, out) {
        n = split(text, parts, "_")
        out = ""
        for (i = 1; i <= n; i++) {
            part = tolower(parts[i])
            out = out toupper(substr(part, 1, 1)) substr(part, 2)
        }
        return out
    }
    function canonical_java_value(value,    parts, n, class_name, enum_name) {
        value = trim_ws(value)
        sub(/^Boolean\.valueOf\(/, "", value)
        sub(/\)$/, "", value)
        value = trim_ws(value)
        if (value == "null") {
            return "none"
        }
        if (value == "true" || value == "false") {
            return value
        }
        n = split(value, parts, ".")
        if (n == 2 && parts[1] ~ /^[A-Za-z0-9_]+$/ && parts[2] ~ /^[A-Z0-9_]+$/) {
            class_name = parts[1]
            enum_name = pascal_from_upper(parts[2])
            return class_name "::" enum_name
        }
        return value
    }
    /_DEP_/ && /=/ && /;/ {
        line = $0
        sub(/^.* final[[:space:]]+/, "", line)
        split(line, assignment, "=")
        lhs = trim_ws(assignment[1])
        rhs = trim_ws(assignment[2])
        sub(/;[[:space:]]*$/, "", rhs)
        n = split(lhs, lhs_parts, /[[:space:]]+/)
        name = lhs_parts[n]
        if (name ~ /_DEP_/) {
            print name "\t" canonical_java_value(rhs)
        }
    }
' "$JAVA_CORE_OPTIONS" | sort -u > "$java_dependency_constants_file"

awk -v const_file="$java_dependency_constants_file" '
    BEGIN {
        while ((getline line < const_file) > 0) {
            split(line, parts, "\t")
            if (parts[1] != "") {
                const_value[parts[1]] = parts[2]
            }
        }
        close(const_file)
    }
    function trim_ws(text) {
        gsub(/^[[:space:]]+/, "", text)
        gsub(/[[:space:]]+$/, "", text)
        return text
    }
    function extract_id(text,    id) {
        if (match(text, /"org\.eclipse\.elk[^"]+"/)) {
            id = substr(text, RSTART + 1, RLENGTH - 2)
            return id
        }
        return ""
    }
    function pascal_from_upper(text,    n, parts, i, part, out) {
        n = split(text, parts, "_")
        out = ""
        for (i = 1; i <= n; i++) {
            part = tolower(parts[i])
            out = out toupper(substr(part, 1, 1)) substr(part, 2)
        }
        return out
    }
    function canonical_java_value(value,    parts, n, class_name, enum_name) {
        value = trim_ws(value)
        sub(/^Boolean\.valueOf\(/, "", value)
        sub(/\)$/, "", value)
        value = trim_ws(value)
        if (value == "null") {
            return "none"
        }
        if (value == "true" || value == "false") {
            return value
        }
        n = split(value, parts, ".")
        if (n == 2 && parts[1] ~ /^[A-Za-z0-9_]+$/ && parts[2] ~ /^[A-Z0-9_]+$/) {
            class_name = parts[1]
            enum_name = pascal_from_upper(parts[2])
            return class_name "::" enum_name
        }
        return value
    }
    /registry\.addDependency[[:space:]]*\(/ {
        if ((getline source_line) <= 0) {
            next
        }
        if ((getline target_line) <= 0) {
            next
        }
        if ((getline value_line) <= 0) {
            next
        }
        source_id = extract_id(source_line)
        target_id = extract_id(target_line)
        value = trim_ws(value_line)
        sub(/,[[:space:]]*$/, "", value)
        if (value in const_value) {
            value = const_value[value]
        } else {
            value = canonical_java_value(value)
        }
        if (source_id != "" && target_id != "") {
            print source_id "\t" target_id "\t" value
        }
    }
' "$JAVA_CORE_OPTIONS" | sort -u > "$java_dependencies_file"

awk -v map_file="$rust_option_map_file" '
    BEGIN {
        while ((getline line < map_file) > 0) {
            split(line, parts, "\t")
            if (parts[1] != "" && parts[2] != "") {
                option_id_by_name[parts[1]] = parts[2]
            }
        }
        close(map_file)
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
    function resolve_option_id(expr,    value, key) {
        value = remove_ws(expr)
        if (match(value, /^CoreOptions::[A-Z0-9_]+\.id\(\)$/)) {
            key = value
            sub(/^CoreOptions::/, "", key)
            sub(/\.id\(\)$/, "", key)
            return option_id_by_name[key]
        }
        if (match(value, /^"org\.eclipse\.elk/)) {
            gsub(/"/, "", value)
            return value
        }
        return ""
    }
    function canonical_rust_value(value) {
        value = trim_ws(value)
        sub(/,[[:space:]]*$/, "", value)
        if (value == "None") {
            return "none"
        }
        if (value ~ /^Some\(arc_any\(/) {
            sub(/^Some\(arc_any\(/, "", value)
            sub(/\)\)$/, "", value)
        }
        value = trim_ws(value)
        return value
    }
    /registry\.add_dependency[[:space:]]*\(/ {
        if ((getline source_line) <= 0) {
            next
        }
        if ((getline target_line) <= 0) {
            next
        }
        if ((getline value_line) <= 0) {
            next
        }
        source_expr = source_line
        target_expr = target_line
        sub(/,[[:space:]]*$/, "", source_expr)
        sub(/,[[:space:]]*$/, "", target_expr)
        source_id = resolve_option_id(source_expr)
        target_id = resolve_option_id(target_expr)
        value = canonical_rust_value(value_line)
        if (source_id != "" && target_id != "") {
            print source_id "\t" target_id "\t" value
        }
    }
' "$RUST_CORE_META" | sort -u > "$rust_dependencies_file"

awk -F '\t' '
    FNR == NR {
        rust_keys[$1 "\t" $2] = 1
        next
    }
    !($1 "\t" $2 in rust_keys) {
        print $0
    }
' "$rust_dependencies_file" "$java_dependencies_file" > "$java_only_file"

awk -F '\t' '
    FNR == NR {
        java_keys[$1 "\t" $2] = 1
        next
    }
    !($1 "\t" $2 in java_keys) {
        print $0
    }
' "$java_dependencies_file" "$rust_dependencies_file" > "$rust_only_file"

awk -F '\t' '
    FNR == NR {
        java_values[$1 "\t" $2] = $3
        next
    }
    {
        key = $1 "\t" $2
        if (key in java_values && java_values[key] != $3) {
            print $1 "\t" $2 "\t" java_values[key] "\t" $3
        }
    }
' "$java_dependencies_file" "$rust_dependencies_file" > "$value_mismatch_file"

java_count="$(wc -l < "$java_dependencies_file" | tr -d ' ')"
rust_count="$(wc -l < "$rust_dependencies_file" | tr -d ' ')"
java_only_count="$(wc -l < "$java_only_file" | tr -d ' ')"
rust_only_count="$(wc -l < "$rust_only_file" | tr -d ' ')"
value_mismatch_count="$(wc -l < "$value_mismatch_file" | tr -d ' ')"

status="ok"
if [ "$java_only_count" -gt 0 ] || [ "$rust_only_count" -gt 0 ] || [ "$value_mismatch_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Core Option Dependency Parity"
    echo
    echo "- status: $status"
    echo "- java dependencies: $java_count"
    echo "- rust dependencies: $rust_count"
    echo "- java-only dependencies: $java_only_count"
    echo "- rust-only dependencies: $rust_only_count"
    echo "- value mismatches: $value_mismatch_count"
    echo
    echo "## Java-Only Dependencies"
    if [ "$java_only_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` -> `%s` (value=%s)\n", $1, $2, $3)}' "$java_only_file"
    fi
    echo
    echo "## Rust-Only Dependencies"
    if [ "$rust_only_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` -> `%s` (value=%s)\n", $1, $2, $3)}' "$rust_only_file"
    fi
    echo
    echo "## Value Mismatches"
    if [ "$value_mismatch_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` -> `%s` (java=%s, rust=%s)\n", $1, $2, $3, $4)}' "$value_mismatch_file"
    fi
} > "$REPORT_FILE"

echo "wrote $REPORT_FILE"

if [ "$status" != "ok" ] && [ "$STRICT_MODE" = "true" ]; then
    exit 1
fi
