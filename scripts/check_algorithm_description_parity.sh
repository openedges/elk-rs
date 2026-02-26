#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
REPORT_FILE="${1:-parity/algorithm_description_parity.md}"
STRICT_MODE="${ALGORITHM_DESCRIPTION_PARITY_STRICT:-false}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-description-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

java_file="$tmp_dir/java_algorithm_description.tsv"
rust_map_file="$tmp_dir/rust_algo_map.tsv"
rust_raw_file="$tmp_dir/rust_algorithm_description.raw.tsv"
rust_file="$tmp_dir/rust_algorithm_description.tsv"

java_ids="$tmp_dir/java_ids.txt"
rust_ids="$tmp_dir/rust_ids.txt"
missing_ids_file="$tmp_dir/missing_ids.txt"
extra_ids_file="$tmp_dir/extra_ids.txt"
mismatch_file="$tmp_dir/description_mismatches.tsv"

# Java: id -> description (normalized)
(rg --files "$JAVA_SOURCES_ROOT" -g '*Options.java' || true) \
    | while IFS= read -r file; do
        [ -f "$file" ] || continue
        awk '
            function trim_ws(text) {
                gsub(/^[[:space:]]+/, "", text)
                gsub(/[[:space:]]+$/, "", text)
                return text
            }
            function normalize_text(text) {
                gsub(/\\\"/, "\"", text)
                gsub(/\\'\''/, "'\''", text)
                gsub(/\\\\/, "\\", text)
                gsub(/[[:space:]]+/, " ", text)
                return trim_ws(text)
            }
            function read_string(text,    value) {
                if (!match(text, /"([^"\\]|\\.)*"/)) {
                    return ""
                }
                value = substr(text, RSTART + 1, RLENGTH - 2)
                return normalize_text(value)
            }
            function reset_state() {
                id = ""
                desc = ""
                in_block = 0
            }
            function flush() {
                if (id != "") {
                    print id "\t" desc
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
            in_block && /\.description\(/ {
                desc = read_string($0)
                next
            }
            in_block && /\.create\(\)/ {
                flush()
            }
            END {
                flush()
            }
        ' "$file"
    done | sort -u > "$java_file"

# Rust struct-name -> ALGORITHM_ID map from options.rs
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

# Rust: id -> description (normalized)
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
            function normalize_text(text) {
                gsub(/\\\"/, "\"", text)
                gsub(/\\'\''/, "'\''", text)
                gsub(/\\\\/, "\\", text)
                gsub(/[[:space:]]+/, " ", text)
                return trim_ws(text)
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
            function collect_quoted(text,    out, pos, m, s) {
                out = ""
                while (match(text, /"([^"\\]|\\.)*"/)) {
                    s = substr(text, RSTART + 1, RLENGTH - 2)
                    out = out s
                    text = substr(text, RSTART + RLENGTH)
                }
                return normalize_text(out)
            }
            function print_record(var,    id) {
                if (!(var in algorithm_by_var)) {
                    return
                }
                id = algorithm_by_var[var]
                if (id == "") {
                    return
                }
                print id "\t" description_by_var[var]
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
                if (line ~ /^[[:space:]]*\.[[:space:]]*set_description[[:space:]]*\(/) {
                    target_var = chain_var
                } else if (line ~ /[A-Za-z_][A-Za-z0-9_]*[[:space:]]*\.[[:space:]]*set_description[[:space:]]*\(/) {
                    target_var = line
                    sub(/^[[:space:]]*/, "", target_var)
                    sub(/[[:space:]]*\..*$/, "", target_var)
                    chain_var = target_var
                }

                if (target_var != "" && (target_var in algorithm_by_var)) {
                    capture_var = target_var
                    capture_mode = 1
                    capture_buf = line
                } else if (capture_mode == 1) {
                    capture_buf = capture_buf " " line
                }

                desc_capture_done = 0
                if (line ~ /set_description[[:space:]]*\(.*\)[[:space:]]*(,|\.|;)[[:space:]]*$/ ||
                    line ~ /set_description[[:space:]]*\(.*\)[[:space:]]*$/) {
                    desc_capture_done = 1
                }
                if (line ~ /^[[:space:]]*\)+[[:space:]]*(,|\.|;)[[:space:]]*$/ ||
                    line ~ /^[[:space:]]*\)+[[:space:]]*$/) {
                    desc_capture_done = 1
                }

                if (capture_mode == 1 && desc_capture_done == 1) {
                    desc_text = capture_buf
                    sub(/^.*set_description[[:space:]]*\(/, "", desc_text)
                    sub(/\)+[[:space:]]*(,|\.|;)[[:space:]]*$/, "", desc_text)
                    sub(/\)+[[:space:]]*$/, "", desc_text)
                    description_by_var[capture_var] = collect_quoted(desc_text)
                    capture_mode = 0
                    capture_var = ""
                    capture_buf = ""
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
    done | sort -u > "$rust_raw_file"

awk -F '\t' '
    {
        id = $1
        desc = $2
        if (!(id in best) || length(desc) > length(best[id])) {
            best[id] = desc
        }
    }
    END {
        for (id in best) {
            print id "\t" best[id]
        }
    }
' "$rust_raw_file" | sort -u > "$rust_file"

cut -f1 "$java_file" | sort -u > "$java_ids"
cut -f1 "$rust_file" | sort -u > "$rust_ids"

comm -23 "$java_ids" "$rust_ids" > "$missing_ids_file"
comm -13 "$java_ids" "$rust_ids" > "$extra_ids_file"

awk -F '\t' '
    FNR == NR {
        java[$1] = $2
        next
    }
    {
        id = $1
        rust_desc = $2
        if (id in java) {
            java_desc = java[id]
            if (java_desc != rust_desc) {
                print id "\t" java_desc "\t" rust_desc
            }
        }
    }
' "$java_file" "$rust_file" > "$mismatch_file"

java_count="$(wc -l < "$java_ids" | tr -d ' ')"
rust_count="$(wc -l < "$rust_ids" | tr -d ' ')"
missing_count="$(wc -l < "$missing_ids_file" | tr -d ' ')"
extra_count="$(wc -l < "$extra_ids_file" | tr -d ' ')"
mismatch_count="$(wc -l < "$mismatch_file" | tr -d ' ')"

status="ok"
if [ "$missing_count" -gt 0 ] || [ "$extra_count" -gt 0 ] || [ "$mismatch_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm Description Parity"
    echo
    echo "- status: $status"
    echo "- java algorithms: $java_count"
    echo "- rust algorithms: $rust_count"
    echo "- missing in rust: $missing_count"
    echo "- extra in rust: $extra_count"
    echo "- description mismatches: $mismatch_count"
    echo
    echo "## Description Mismatches (id | java | rust)"
    if [ -s "$mismatch_file" ]; then
        while IFS="$(printf '\t')" read -r id java_desc rust_desc; do
            echo "- $id | $java_desc | $rust_desc"
        done < "$mismatch_file"
    else
        echo "- none"
    fi
    echo
    echo "## Missing In Rust"
    if [ -s "$missing_ids_file" ]; then
        while IFS= read -r id; do
            echo "- $id"
        done < "$missing_ids_file"
    else
        echo "- none"
    fi
    echo
    echo "## Extra In Rust"
    if [ -s "$extra_ids_file" ]; then
        while IFS= read -r id; do
            echo "- $id"
        done < "$extra_ids_file"
    else
        echo "- none"
    fi
} > "$REPORT_FILE"

if [ "$STRICT_MODE" = "true" ] && [ "$status" != "ok" ]; then
    echo "algorithm description parity drift detected (strict mode): $REPORT_FILE" >&2
    exit 1
fi

echo "wrote algorithm description parity report: $REPORT_FILE"
