#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
RUST_CORE_DATA_FILE="${RUST_CORE_DATA_FILE:-$RUST_SOURCES_ROOT/org.eclipse.elk.core/src/org/eclipse/elk/core/data/mod.rs}"
REPORT_FILE="${1:-tests/algorithm_option_support_parity.md}"
STRICT_MODE="${ALGORITHM_OPTION_SUPPORT_PARITY_STRICT:-false}"
IGNORE_IDS="${ALGORITHM_OPTION_SUPPORT_IGNORE_IDS:-}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-option-support-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

rust_algo_map_file="$tmp_dir/rust_algo_map.tsv"
java_support_algos_file="$tmp_dir/java_support_algos.txt"
rust_provider_support_algos_file="$tmp_dir/rust_provider_support_algos.txt"
rust_core_support_algos_file="$tmp_dir/rust_core_support_algos.txt"
rust_support_algos_file="$tmp_dir/rust_support_algos.txt"
java_counts_file="$tmp_dir/java_counts.tsv"
rust_counts_file="$tmp_dir/rust_counts.tsv"
combined_counts_file="$tmp_dir/combined_counts.tsv"

java_only_file="$tmp_dir/java_only_algorithms.tsv"
rust_only_file="$tmp_dir/rust_only_algorithms.tsv"
mismatch_file="$tmp_dir/mismatch_algorithms.tsv"
filtered_combined_counts_file="$tmp_dir/filtered_combined_counts.tsv"
ignore_ids_file="$tmp_dir/ignore_ids.txt"
ignored_matches_file="$tmp_dir/ignored_matches.tsv"

printf '%s\n' "$IGNORE_IDS" | tr ',' '\n' | awk 'NF > 0 { gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); if ($0 != "") print $0 }' | sort -u > "$ignore_ids_file"

# Build a struct-name -> ALGORITHM_ID map from Rust option definition files.
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

# Java: collect algorithm ids from addOptionSupport(<algo>, ...).
(rg --files "$JAVA_SOURCES_ROOT" -g '*Options.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            /registry\.addOptionSupport[[:space:]]*\(/ {
                in_call = 1
                next
            }
            in_call {
                if (match($0, /"org\.eclipse\.elk[^"]+"/)) {
                    algo = substr($0, RSTART + 1, RLENGTH - 2)
                    print algo
                    in_call = 0
                    next
                }
                if ($0 ~ /\)[[:space:]]*;/) {
                    in_call = 0
                }
            }
        ' "$file"
    done | sort > "$java_support_algos_file"

# Rust (provider): collect algorithm ids from add_option_support(<algo>, ...).
(rg --files "$RUST_SOURCES_ROOT" -g '*meta_data_provider.rs' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk -v map_file="$rust_algo_map_file" '
            BEGIN {
                while ((getline line < map_file) > 0) {
                    split(line, parts, "\t")
                    if (parts[1] != "" && parts[2] != "") {
                        algo_id_by_struct[parts[1]] = parts[2]
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
            function resolve_algo(token,    expr, n, parts, key) {
                token = remove_ws(token)
                if (token ~ /^"org\.eclipse\.elk/) {
                    gsub(/"/, "", token)
                    return token
                }
                if (token ~ /::ALGORITHM_ID$/) {
                    expr = token
                    sub(/::ALGORITHM_ID$/, "", expr)
                    n = split(expr, parts, /::/)
                    key = parts[n]
                    return algo_id_by_struct[key]
                }
                expr = local_var_expr[token]
                if (expr != "") {
                    n = split(expr, parts, /::/)
                    key = parts[n]
                    return algo_id_by_struct[key]
                }
                return ""
            }
            /let[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[[:space:]]*[A-Za-z_][A-Za-z0-9_:]*::ALGORITHM_ID[[:space:]]*;/ {
                var_name = $0
                sub(/^.*let[[:space:]]+/, "", var_name)
                sub(/[[:space:]]*=.*$/, "", var_name)
                var_name = trim_ws(var_name)

                expr = $0
                sub(/^.*=[[:space:]]*/, "", expr)
                sub(/::ALGORITHM_ID.*$/, "", expr)
                expr = remove_ws(expr)
                local_var_expr[var_name] = expr
            }
            {
                if (!in_call && $0 ~ /registry\.add_option_support[[:space:]]*\(/) {
                    in_call = 1
                    call_text = $0
                } else if (in_call) {
                    call_text = call_text " " $0
                }

                if (in_call && $0 ~ /\)[[:space:]]*;/) {
                    call = call_text
                    sub(/^.*registry\.add_option_support[[:space:]]*\(/, "", call)
                    first_arg = call
                    sub(/,.*/, "", first_arg)
                    algo = resolve_algo(first_arg)
                    if (algo != "") {
                        print algo
                    }
                    in_call = 0
                    call_text = ""
                }
            }
        ' "$file"
    done | sort > "$rust_provider_support_algos_file"

# Rust (core): collect add_known_option_default for core-built algorithms that don't have provider support.
if [ -f "$RUST_CORE_DATA_FILE" ]; then
    awk -v map_file="$rust_algo_map_file" -v provider_file="$rust_provider_support_algos_file" '
        BEGIN {
            while ((getline line < map_file) > 0) {
                split(line, parts, "\t")
                if (parts[1] != "" && parts[2] != "") {
                    algo_id_by_struct[parts[1]] = parts[2]
                }
            }
            close(map_file)
            while ((getline line < provider_file) > 0) {
                if (line != "") {
                    provider_algorithms[line] = 1
                }
            }
            close(provider_file)
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
        function resolve_algo(token,    expr, n, parts, key) {
            token = remove_ws(token)
            if (token ~ /^"org\.eclipse\.elk/) {
                gsub(/"/, "", token)
                return token
            }
            if (token ~ /::ALGORITHM_ID$/) {
                expr = token
                sub(/::ALGORITHM_ID$/, "", expr)
                n = split(expr, parts, /::/)
                key = parts[n]
                return algo_id_by_struct[key]
            }
            return ""
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
        /[A-Za-z_][A-Za-z0-9_]*\.add_known_option_default[[:space:]]*\(/ {
            var_name = $0
            sub(/^[[:space:]]*/, "", var_name)
            sub(/\.add_known_option_default[[:space:]]*\(.*/, "", var_name)
            if (var_name in algorithm_by_var) {
                algo = algorithm_by_var[var_name]
                if (!(algo in provider_algorithms)) {
                    print algo
                }
            }
        }
    ' "$RUST_CORE_DATA_FILE" | sort > "$rust_core_support_algos_file"
else
    : > "$rust_core_support_algos_file"
fi

cat "$rust_provider_support_algos_file" "$rust_core_support_algos_file" | sort > "$rust_support_algos_file"

awk '
    { count[$1] += 1 }
    END {
        for (algo in count) {
            print algo "\t" count[algo]
        }
    }
' "$java_support_algos_file" | sort > "$java_counts_file"

awk '
    { count[$1] += 1 }
    END {
        for (algo in count) {
            print algo "\t" count[algo]
        }
    }
' "$rust_support_algos_file" | sort > "$rust_counts_file"

awk -F '\t' '
    FNR == NR {
        java[$1] = $2
        all[$1] = 1
        next
    }
    {
        rust[$1] = $2
        all[$1] = 1
    }
    END {
        for (algo in all) {
            j = (algo in java) ? java[algo] : 0
            r = (algo in rust) ? rust[algo] : 0
            d = r - j
            print algo "\t" j "\t" r "\t" d
        }
    }
' "$java_counts_file" "$rust_counts_file" | sort > "$combined_counts_file"

awk -F '\t' -v ignore_file="$ignore_ids_file" '
    BEGIN {
        while ((getline line < ignore_file) > 0) {
            if (line != "") {
                ignored[line] = 1
            }
        }
        close(ignore_file)
    }
    !($1 in ignored) {
        print
    }
' "$combined_counts_file" > "$filtered_combined_counts_file"

awk -F '\t' -v ignore_file="$ignore_ids_file" '
    BEGIN {
        while ((getline line < ignore_file) > 0) {
            if (line != "") {
                ignored[line] = 1
            }
        }
        close(ignore_file)
    }
    ($1 in ignored) {
        print
    }
' "$combined_counts_file" > "$ignored_matches_file"

awk -F '\t' '$2 > 0 && $3 == 0 { print }' "$filtered_combined_counts_file" > "$java_only_file"
awk -F '\t' '$2 == 0 && $3 > 0 { print }' "$filtered_combined_counts_file" > "$rust_only_file"
awk -F '\t' '$2 > 0 && $3 > 0 && $2 != $3 { print }' "$filtered_combined_counts_file" > "$mismatch_file"

java_algo_count="$(wc -l < "$java_counts_file" | tr -d ' ')"
rust_algo_count="$(wc -l < "$rust_counts_file" | tr -d ' ')"
java_only_count="$(wc -l < "$java_only_file" | tr -d ' ')"
rust_only_count="$(wc -l < "$rust_only_file" | tr -d ' ')"
mismatch_count="$(wc -l < "$mismatch_file" | tr -d ' ')"
ignored_count="$(wc -l < "$ignored_matches_file" | tr -d ' ')"

status="ok"
if [ "$java_only_count" -gt 0 ] || [ "$rust_only_count" -gt 0 ] || [ "$mismatch_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm Option Support Parity"
    echo
    echo "- status: $status"
    echo "- java algorithms with option support: $java_algo_count"
    echo "- rust algorithms with option support: $rust_algo_count"
    echo "- algorithms only in java support map: $java_only_count"
    echo "- algorithms only in rust support map: $rust_only_count"
    echo "- algorithms with count mismatch: $mismatch_count"
    echo "- ignored algorithms: $ignored_count"
    echo
    echo "## Per-Algorithm Counts (algo | java | rust | delta)"
    if [ -s "$filtered_combined_counts_file" ]; then
        awk -F '\t' '{printf("- `%s` | %s | %s | %+d\n", $1, $2, $3, $4)}' "$filtered_combined_counts_file"
    else
        echo "- none"
    fi
    echo
    echo "## Java-Only Algorithms"
    if [ "$java_only_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` (java=%s, rust=%s)\n", $1, $2, $3)}' "$java_only_file"
    fi
    echo
    echo "## Rust-Only Algorithms"
    if [ "$rust_only_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` (java=%s, rust=%s)\n", $1, $2, $3)}' "$rust_only_file"
    fi
    echo
    echo "## Count Mismatch Algorithms"
    if [ "$mismatch_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` (java=%s, rust=%s, delta=%+d)\n", $1, $2, $3, $4)}' "$mismatch_file"
    fi
    echo
    echo "## Ignored Algorithms"
    if [ "$ignored_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` (java=%s, rust=%s, delta=%+d)\n", $1, $2, $3, $4)}' "$ignored_matches_file"
    fi
} > "$REPORT_FILE"

echo "wrote $REPORT_FILE"

if [ "$status" != "ok" ] && [ "$STRICT_MODE" = "true" ]; then
    exit 1
fi
