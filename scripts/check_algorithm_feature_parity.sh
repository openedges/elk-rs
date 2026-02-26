#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
REPORT_FILE="${1:-parity/algorithm_feature_parity.md}"
STRICT_MODE="${ALGORITHM_FEATURE_PARITY_STRICT:-false}"
IGNORE_IDS="${ALGORITHM_FEATURE_IGNORE_IDS:-}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-feature-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

rust_algo_map_file="$tmp_dir/rust_algo_map.tsv"
java_pairs_file="$tmp_dir/java_pairs.tsv"
rust_pairs_file="$tmp_dir/rust_pairs.tsv"
filtered_java_pairs_file="$tmp_dir/filtered_java_pairs.tsv"
filtered_rust_pairs_file="$tmp_dir/filtered_rust_pairs.tsv"
java_only_pairs_file="$tmp_dir/java_only_pairs.tsv"
rust_only_pairs_file="$tmp_dir/rust_only_pairs.tsv"
ignore_ids_file="$tmp_dir/ignore_ids.txt"
ignored_pairs_file="$tmp_dir/ignored_pairs.tsv"
all_algorithms_file="$tmp_dir/all_algorithms.txt"
per_algo_counts_file="$tmp_dir/per_algo_counts.tsv"

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

# Java: extract (algorithm, feature) pairs from supportedFeatures().
(rg --files "$JAVA_SOURCES_ROOT" -g '*Options.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            function extract_features(algo, text,    feature) {
                while (match(text, /GraphFeature\.[A-Z_]+/)) {
                    feature = substr(text, RSTART, RLENGTH)
                    sub(/^GraphFeature\./, "", feature)
                    print algo "\t" feature
                    text = substr(text, RSTART + RLENGTH)
                }
            }
            /\.[[:space:]]*id\("org\.eclipse\.elk/ {
                if (match($0, /"org\.eclipse\.elk[^"]+"/)) {
                    current_algo = substr($0, RSTART + 1, RLENGTH - 2)
                }
            }
            /\.supportedFeatures[[:space:]]*\(EnumSet\.of\(/ {
                in_features = 1
                features_text = $0
                if ($0 ~ /\)\)/) {
                    extract_features(current_algo, features_text)
                    in_features = 0
                    features_text = ""
                }
                next
            }
            in_features {
                features_text = features_text " " $0
                if ($0 ~ /\)\)/) {
                    extract_features(current_algo, features_text)
                    in_features = 0
                    features_text = ""
                }
            }
        ' "$file"
    done | sort -u > "$java_pairs_file"

# Rust: extract (algorithm, feature) pairs from add_supported_feature().
(rg --files "$RUST_SOURCES_ROOT" -g '*.rs' || true) \
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
            function resolve_algo(expr,    value, n, parts, key) {
                value = remove_ws(expr)
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
            function to_java_feature(rust_name) {
                if (rust_name == "SelfLoops") {
                    return "SELF_LOOPS"
                } else if (rust_name == "InsideSelfLoops") {
                    return "INSIDE_SELF_LOOPS"
                } else if (rust_name == "MultiEdges") {
                    return "MULTI_EDGES"
                } else if (rust_name == "EdgeLabels") {
                    return "EDGE_LABELS"
                } else if (rust_name == "Ports") {
                    return "PORTS"
                } else if (rust_name == "Compound") {
                    return "COMPOUND"
                } else if (rust_name == "Clusters") {
                    return "CLUSTERS"
                } else if (rust_name == "Disconnected") {
                    return "DISCONNECTED"
                }
                return ""
            }
            /let[[:space:]]+(mut[[:space:]]+)?[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[[:space:]]*LayoutAlgorithmData::new\(/ {
                line = $0
                sub(/^.*let[[:space:]]+(mut[[:space:]]+)?/, "", line)
                var_name = line
                sub(/[[:space:]]*=.*$/, "", var_name)
                var_name = trim_ws(var_name)

                algo_expr = $0
                sub(/^.*LayoutAlgorithmData::new\(/, "", algo_expr)
                sub(/\).*/, "", algo_expr)
                algo_expr = trim_ws(algo_expr)
                algo_id = resolve_algo(algo_expr)
                if (algo_id != "") {
                    algorithm_by_var[var_name] = algo_id
                    chain_var = var_name
                }
                next
            }
            /^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]*$/ {
                maybe_var = trim_ws($0)
                if (maybe_var in algorithm_by_var) {
                    chain_var = maybe_var
                }
            }
            {
                line = $0
                if (line ~ /^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\./) {
                    var_name = line
                    sub(/^[[:space:]]*/, "", var_name)
                    sub(/[[:space:]]*\..*$/, "", var_name)
                    if (var_name in algorithm_by_var) {
                        chain_var = var_name
                    }
                }
                if (line ~ /^[[:space:]]*\.[[:space:]]*add_supported_feature[[:space:]]*\(GraphFeature::[A-Za-z]+/) {
                    var_name = chain_var
                } else if (line ~ /[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\.[[:space:]]*add_supported_feature[[:space:]]*\(GraphFeature::[A-Za-z]+/) {
                    var_name = line
                    sub(/^[[:space:]]*/, "", var_name)
                    sub(/[[:space:]]*\..*$/, "", var_name)
                    chain_var = var_name
                } else {
                    if (line ~ /;[[:space:]]*$/) {
                        chain_var = ""
                    }
                    next
                }

                if (!(var_name in algorithm_by_var)) {
                    next
                }
                algo = algorithm_by_var[var_name]
                feature = line
                sub(/^.*GraphFeature::/, "", feature)
                sub(/\).*/, "", feature)
                feature = trim_ws(feature)
                java_feature = to_java_feature(feature)
                if (algo != "" && java_feature != "") {
                    print algo "\t" java_feature
                }
                if (line ~ /;[[:space:]]*$/) {
                    chain_var = ""
                }
            }
        ' "$file"
    done | sort -u > "$rust_pairs_file"

awk -v ignore_file="$ignore_ids_file" '
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
' "$java_pairs_file" > "$filtered_java_pairs_file"

awk -v ignore_file="$ignore_ids_file" '
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
' "$rust_pairs_file" > "$filtered_rust_pairs_file"

awk -F '\t' '
    FNR == NR {
        rust[$1 "\t" $2] = 1
        next
    }
    !($1 "\t" $2 in rust) {
        print
    }
' "$filtered_rust_pairs_file" "$filtered_java_pairs_file" > "$java_only_pairs_file"

awk -F '\t' '
    FNR == NR {
        java[$1 "\t" $2] = 1
        next
    }
    !($1 "\t" $2 in java) {
        print
    }
' "$filtered_java_pairs_file" "$filtered_rust_pairs_file" > "$rust_only_pairs_file"

cat "$filtered_java_pairs_file" "$filtered_rust_pairs_file" | awk -F '\t' '{print $1}' | sort -u > "$all_algorithms_file"

awk -F '\t' '
    FNR == NR {
        java[$1] += 1
        next
    }
    {
        rust[$1] += 1
    }
    END {
        for (algo in java) {
            all[algo] = 1
        }
        for (algo in rust) {
            all[algo] = 1
        }
        for (algo in all) {
            j = (algo in java) ? java[algo] : 0
            r = (algo in rust) ? rust[algo] : 0
            d = r - j
            print algo "\t" j "\t" r "\t" d
        }
    }
' "$filtered_java_pairs_file" "$filtered_rust_pairs_file" | sort > "$per_algo_counts_file"

awk -v ignore_file="$ignore_ids_file" '
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
' "$java_pairs_file" > "$ignored_pairs_file"

java_only_count="$(wc -l < "$java_only_pairs_file" | tr -d ' ')"
rust_only_count="$(wc -l < "$rust_only_pairs_file" | tr -d ' ')"
ignored_count="$(wc -l < "$ignored_pairs_file" | tr -d ' ')"
java_pair_count="$(wc -l < "$filtered_java_pairs_file" | tr -d ' ')"
rust_pair_count="$(wc -l < "$filtered_rust_pairs_file" | tr -d ' ')"
algo_count="$(wc -l < "$per_algo_counts_file" | tr -d ' ')"

status="ok"
if [ "$java_only_count" -gt 0 ] || [ "$rust_only_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm Feature Parity"
    echo
    echo "- status: $status"
    echo "- compared algorithms: $algo_count"
    echo "- java feature pairs: $java_pair_count"
    echo "- rust feature pairs: $rust_pair_count"
    echo "- java-only feature pairs: $java_only_count"
    echo "- rust-only feature pairs: $rust_only_count"
    echo "- ignored pairs: $ignored_count"
    echo
    echo "## Per-Algorithm Feature Counts (algo | java | rust | delta)"
    if [ -s "$per_algo_counts_file" ]; then
        awk -F '\t' '{printf("- `%s` | %s | %s | %+d\n", $1, $2, $3, $4)}' "$per_algo_counts_file"
    else
        echo "- none"
    fi
    echo
    echo "## Java-Only Feature Pairs"
    if [ "$java_only_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` -> `%s`\n", $1, $2)}' "$java_only_pairs_file"
    fi
    echo
    echo "## Rust-Only Feature Pairs"
    if [ "$rust_only_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` -> `%s`\n", $1, $2)}' "$rust_only_pairs_file"
    fi
    echo
    echo "## Ignored Feature Pairs"
    if [ "$ignored_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{printf("- `%s` -> `%s`\n", $1, $2)}' "$ignored_pairs_file"
    fi
} > "$REPORT_FILE"

echo "wrote $REPORT_FILE"

if [ "$status" != "ok" ] && [ "$STRICT_MODE" = "true" ]; then
    exit 1
fi
