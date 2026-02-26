#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
REPORT_FILE="${1:-parity/algorithm_id_parity.md}"
STRICT_MODE="${ALGORITHM_ID_PARITY_STRICT:-false}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-id-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

java_ids_file="$tmp_dir/java_algorithm_ids.txt"
rust_ids_file="$tmp_dir/rust_algorithm_ids.txt"
missing_ids_file="$tmp_dir/missing_algorithm_ids.txt"
extra_ids_file="$tmp_dir/extra_algorithm_ids.txt"

(rg -o 'ALGORITHM_ID\s*=\s*"org\.eclipse\.elk[^"]+"' "$JAVA_SOURCES_ROOT" -g '*.java' || true) \
    | sed -E 's/.*"([^"]+)".*/\1/' \
    | sort -u \
    > "$java_ids_file"

(rg -o "ALGORITHM_ID\\s*:\\s*&[^=]*=\\s*\"org\\.eclipse\\.elk[^\"]+\"" "$RUST_SOURCES_ROOT" -g '*.rs' || true) \
    | sed -E 's/.*"([^"]+)".*/\1/' \
    | sort -u \
    > "$rust_ids_file"

comm -23 "$java_ids_file" "$rust_ids_file" > "$missing_ids_file"
comm -13 "$java_ids_file" "$rust_ids_file" > "$extra_ids_file"

missing_count="$(wc -l < "$missing_ids_file" | tr -d ' ')"
extra_count="$(wc -l < "$extra_ids_file" | tr -d ' ')"

status="ok"
if [ "$missing_count" -gt 0 ] || [ "$extra_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm ID Parity"
    echo
    echo "- status: $status"
    echo "- java algorithm ids: $(wc -l < "$java_ids_file" | tr -d ' ')"
    echo "- rust algorithm ids: $(wc -l < "$rust_ids_file" | tr -d ' ')"
    echo "- missing algorithm ids in rust: $missing_count"
    echo "- extra algorithm ids in rust: $extra_count"
    echo
    echo "## Missing Algorithm IDs"
    if [ "$missing_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$missing_ids_file"
    fi
    echo
    echo "## Extra Algorithm IDs"
    if [ "$extra_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$extra_ids_file"
    fi
} > "$REPORT_FILE"

echo "wrote $REPORT_FILE"

if [ "$status" != "ok" ] && [ "$STRICT_MODE" = "true" ]; then
    exit 1
fi
