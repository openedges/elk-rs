#!/usr/bin/env sh
set -eu

JAVA_SOURCES_ROOT="${JAVA_SOURCES_ROOT:-external/elk/plugins}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
REPORT_FILE="${1:-tests/algorithm_category_parity.md}"
STRICT_MODE="${ALGORITHM_CATEGORY_PARITY_STRICT:-false}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/algorithm-category-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

java_categories_file="$tmp_dir/java_algorithm_categories.txt"
rust_categories_file="$tmp_dir/rust_algorithm_categories.txt"
missing_categories_file="$tmp_dir/missing_algorithm_categories.txt"
extra_categories_file="$tmp_dir/extra_algorithm_categories.txt"

(rg -o '\.category\("org\.eclipse\.elk[^"]+"\)' "$JAVA_SOURCES_ROOT" -g '*Options.java' || true) \
    | sed -E 's/.*"([^"]+)".*/\1/' \
    | sort -u \
    > "$java_categories_file"

(rg -o 'set_category_id\(Some\("org\.eclipse\.elk[^"]+"\)\)' "$RUST_SOURCES_ROOT" -g '*.rs' || true) \
    | sed -E 's/.*Some\("([^"]+)"\).*/\1/' \
    | sort -u \
    > "$rust_categories_file"

comm -23 "$java_categories_file" "$rust_categories_file" > "$missing_categories_file"
comm -13 "$java_categories_file" "$rust_categories_file" > "$extra_categories_file"

missing_count="$(wc -l < "$missing_categories_file" | tr -d ' ')"
extra_count="$(wc -l < "$extra_categories_file" | tr -d ' ')"

status="ok"
if [ "$missing_count" -gt 0 ] || [ "$extra_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Algorithm Category Parity"
    echo
    echo "- status: $status"
    echo "- java categories: $(wc -l < "$java_categories_file" | tr -d ' ')"
    echo "- rust categories: $(wc -l < "$rust_categories_file" | tr -d ' ')"
    echo "- missing categories in rust: $missing_count"
    echo "- extra categories in rust: $extra_count"
    echo
    echo "## Missing Categories"
    if [ "$missing_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$missing_categories_file"
    fi
    echo
    echo "## Extra Categories"
    if [ "$extra_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$extra_categories_file"
    fi
} > "$REPORT_FILE"

echo "wrote $REPORT_FILE"

if [ "$status" != "ok" ] && [ "$STRICT_MODE" = "true" ]; then
    exit 1
fi
