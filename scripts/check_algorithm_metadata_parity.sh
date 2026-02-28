#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
REPORT_FILE="${1:-tests/algorithm_metadata_parity.md}"
STRICT_MODE="${ALGORITHM_METADATA_PARITY_STRICT:-false}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-metadata-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

java_meta_file="$tmp_dir/java_algorithm_metadata.tsv"
rust_map_file="$tmp_dir/rust_algo_map.tsv"
rust_meta_file="$tmp_dir/rust_algorithm_metadata.tsv"
java_ids_file="$tmp_dir/java_ids.txt"
rust_ids_file="$tmp_dir/rust_ids.txt"
missing_ids_file="$tmp_dir/missing_ids.txt"
extra_ids_file="$tmp_dir/extra_ids.txt"
mismatch_file="$tmp_dir/metadata_mismatches.tsv"

# Java metadata: id, category, melkBundleName, definingBundleId, imagePath
(rg --files "$JAVA_SOURCES_ROOT" -g '*Options.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            function reset_state() {
                id = ""
                category = ""
                melk = ""
                bundle = ""
                image = ""
                in_block = 0
            }
            function read_string(text,    value) {
                if (!match(text, /"[^"]+"/)) {
                    return ""
                }
                value = substr(text, RSTART + 1, RLENGTH - 2)
                return value
            }
            function flush() {
                if (id != "") {
                    print id "\t" category "\t" melk "\t" bundle "\t" image
                }
                reset_state()
            }
            /LayoutAlgorithmData\.Builder\(\)/ {
                flush()
                in_block = 1
                next
            }
            in_block && /\.id\("org\.eclipse\.elk/ {
                id = read_string($0)
                next
            }
            in_block && /\.category\(/ {
                category = read_string($0)
                next
            }
            in_block && /\.melkBundleName\(/ {
                if ($0 ~ /null/) {
                    melk = ""
                } else {
                    melk = read_string($0)
                }
                next
            }
            in_block && /\.definingBundleId\(/ {
                bundle = read_string($0)
                next
            }
            in_block && /\.imagePath\(/ {
                image = read_string($0)
                next
            }
            in_block && /\.create\(\)/ {
                flush()
            }
            END {
                flush()
            }
        ' "$file"
    done | sort -u > "$java_meta_file"

# Rust struct-name -> ALGORITHM_ID map from option definition files.
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
    done | sort -u > "$rust_map_file"

# Rust metadata: id, category, bundle_name, defining_bundle_id, preview_image_path
(rg --files "$RUST_SOURCES_ROOT" -g '*.rs' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk -v map_file="$rust_map_file" '
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
            function parse_some_string(text,    value) {
                if (text ~ /None/) {
                    return ""
                }
                if (match(text, /Some\("([^"]*)"\)/)) {
                    value = substr(text, RSTART, RLENGTH)
                    sub(/^Some\("/, "", value)
                    sub(/"\)$/, "", value)
                    return value
                }
                return ""
            }
            function print_record(var,    id) {
                if (!(var in algorithm_by_var)) {
                    return
                }
                id = algorithm_by_var[var]
                if (id == "") {
                    return
                }
                print id "\t" category_by_var[var] "\t" bundle_by_var[var] "\t" defining_by_var[var] "\t" image_by_var[var]
                printed[var] = 1
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

            {
                line = $0
                if (line ~ /^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]*$/) {
                    maybe_var = trim_ws(line)
                    if (maybe_var in algorithm_by_var) {
                        chain_var = maybe_var
                    }
                }
                if (line ~ /^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\./) {
                    var_name = line
                    sub(/^[[:space:]]*/, "", var_name)
                    sub(/[[:space:]]*\..*$/, "", var_name)
                    if (var_name in algorithm_by_var) {
                        chain_var = var_name
                    }
                }

                target_var = ""
                if (line ~ /^[[:space:]]*\.[[:space:]]*set_category_id[[:space:]]*\(/ ||
                    line ~ /^[[:space:]]*\.[[:space:]]*set_bundle_name[[:space:]]*\(/ ||
                    line ~ /^[[:space:]]*\.[[:space:]]*set_defining_bundle_id[[:space:]]*\(/ ||
                    line ~ /^[[:space:]]*\.[[:space:]]*set_preview_image_path[[:space:]]*\(/) {
                    target_var = chain_var
                } else if (line ~ /[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\.[[:space:]]*set_category_id[[:space:]]*\(/ ||
                           line ~ /[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\.[[:space:]]*set_bundle_name[[:space:]]*\(/ ||
                           line ~ /[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\.[[:space:]]*set_defining_bundle_id[[:space:]]*\(/ ||
                           line ~ /[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\.[[:space:]]*set_preview_image_path[[:space:]]*\(/) {
                    target_var = line
                    sub(/^[[:space:]]*/, "", target_var)
                    sub(/[[:space:]]*\..*$/, "", target_var)
                    chain_var = target_var
                }

                if (target_var != "" && (target_var in algorithm_by_var)) {
                    if (line ~ /set_category_id[[:space:]]*\(/) {
                        category_by_var[target_var] = parse_some_string(line)
                    } else if (line ~ /set_bundle_name[[:space:]]*\(/) {
                        bundle_by_var[target_var] = parse_some_string(line)
                    } else if (line ~ /set_defining_bundle_id[[:space:]]*\(/) {
                        defining_by_var[target_var] = parse_some_string(line)
                    } else if (line ~ /set_preview_image_path[[:space:]]*\(/) {
                        image_by_var[target_var] = parse_some_string(line)
                    }
                }

                if (line ~ /register_algorithm[[:space:]]*\([[:space:]]*[A-Za-z_][A-Za-z0-9_]*/) {
                    var_name = line
                    sub(/^.*register_algorithm[[:space:]]*\(/, "", var_name)
                    sub(/[[:space:]]*\).*/, "", var_name)
                    var_name = trim_ws(var_name)
                    print_record(var_name)
                } else if (line ~ /register_layout_algorithm[[:space:]]*\([[:space:]]*[A-Za-z_][A-Za-z0-9_]*/) {
                    var_name = line
                    sub(/^.*register_layout_algorithm[[:space:]]*\(/, "", var_name)
                    sub(/[[:space:]]*\).*/, "", var_name)
                    var_name = trim_ws(var_name)
                    print_record(var_name)
                }

                if (line ~ /;[[:space:]]*$/) {
                    chain_var = ""
                }
            }
        ' "$file"
    done | sort -u > "$rust_meta_file"

cut -f1 "$java_meta_file" | sort -u > "$java_ids_file"
cut -f1 "$rust_meta_file" | sort -u > "$rust_ids_file"

comm -23 "$java_ids_file" "$rust_ids_file" > "$missing_ids_file"
comm -13 "$java_ids_file" "$rust_ids_file" > "$extra_ids_file"

awk -F '\t' '
    FNR == NR {
        java_category[$1] = $2
        java_bundle[$1] = $3
        java_defining[$1] = $4
        java_image[$1] = $5
        next
    }
    {
        id = $1
        if (!(id in java_category)) {
            next
        }
        rust_category = $2
        rust_bundle = $3
        rust_defining = $4
        rust_image = $5

        if (java_category[id] != rust_category) {
            print id "\tcategory\t" java_category[id] "\t" rust_category
        }
        if (java_bundle[id] != rust_bundle) {
            print id "\tbundle_name\t" java_bundle[id] "\t" rust_bundle
        }
        if (java_defining[id] != rust_defining) {
            print id "\tdefining_bundle_id\t" java_defining[id] "\t" rust_defining
        }
        if (java_image[id] != rust_image) {
            print id "\timage_path\t" java_image[id] "\t" rust_image
        }
    }
' "$java_meta_file" "$rust_meta_file" > "$mismatch_file"

missing_count="$(wc -l < "$missing_ids_file" | tr -d ' ')"
extra_count="$(wc -l < "$extra_ids_file" | tr -d ' ')"
mismatch_count="$(wc -l < "$mismatch_file" | tr -d ' ')"

status="ok"
if [ "$missing_count" -gt 0 ] || [ "$extra_count" -gt 0 ] || [ "$mismatch_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm Metadata Parity"
    echo
    echo "- status: $status"
    echo "- java algorithms: $(wc -l < "$java_ids_file" | tr -d ' ')"
    echo "- rust algorithms: $(wc -l < "$rust_ids_file" | tr -d ' ')"
    echo "- missing algorithms in rust: $missing_count"
    echo "- extra algorithms in rust: $extra_count"
    echo "- metadata field mismatches: $mismatch_count"
    echo
    echo "## Missing Algorithms"
    if [ "$missing_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$missing_ids_file"
    fi
    echo
    echo "## Extra Algorithms"
    if [ "$extra_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$extra_ids_file"
    fi
    echo
    echo "## Field Mismatches"
    if [ "$mismatch_count" -eq 0 ]; then
        echo "- none"
    else
        awk -F '\t' '{ printf("- %s | %s | java=`%s` | rust=`%s`\n", $1, $2, $3, $4) }' "$mismatch_file"
    fi
} > "$REPORT_FILE"

echo "wrote $REPORT_FILE"

if [ "$status" != "ok" ] && [ "$STRICT_MODE" = "true" ]; then
    exit 1
fi
