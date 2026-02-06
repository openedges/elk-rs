#!/usr/bin/env sh
set -eu

JAVA_CORE_OPTIONS="${JAVA_CORE_OPTIONS:-external/elk/plugins/org.eclipse.elk.core/src-gen/org/eclipse/elk/core/options/CoreOptions.java}"
RUST_CORE_OPTIONS="${RUST_CORE_OPTIONS:-plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/options/core_options.rs}"
RUST_CORE_META="${RUST_CORE_META:-plugins/org.eclipse.elk.core/src/org/eclipse/elk/core/options/core_options_meta.rs}"
RUST_SOURCES_ROOT="${RUST_SOURCES_ROOT:-plugins}"
REPORT_FILE="${1:-perf/core_options_parity.md}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/core-options-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

java_property_ids_file="$tmp_dir/java_property_ids.txt"
rust_property_ids_file="$tmp_dir/rust_property_ids.txt"
missing_property_ids_file="$tmp_dir/missing_property_ids.txt"
extra_property_ids_file="$tmp_dir/extra_property_ids.txt"

java_category_ids_file="$tmp_dir/java_category_ids.txt"
rust_category_ids_file="$tmp_dir/rust_category_ids.txt"
missing_category_ids_file="$tmp_dir/missing_category_ids.txt"
extra_category_ids_file="$tmp_dir/extra_category_ids.txt"

rust_provider_category_ids_file="$tmp_dir/rust_provider_category_ids.txt"
non_qualified_provider_category_ids_file="$tmp_dir/non_qualified_provider_category_ids.txt"

sed '/public void apply/,$d' "$JAVA_CORE_OPTIONS" \
    | rg -o '"org\.eclipse\.elk[^"]+"' \
    | tr -d '"' \
    | sort -u \
    > "$java_property_ids_file"

rg -o '"org\.eclipse\.elk[^"]+"' "$RUST_CORE_OPTIONS" \
    | tr -d '"' \
    | sort -u \
    > "$rust_property_ids_file"

comm -23 "$java_property_ids_file" "$rust_property_ids_file" > "$missing_property_ids_file"
comm -13 "$java_property_ids_file" "$rust_property_ids_file" > "$extra_property_ids_file"

awk '
    /new LayoutCategoryData.Builder\(\)/ { in_category = 1; next }
    in_category && /\.id\("org\.eclipse\.elk/ {
        if (match($0, /"org\.eclipse\.elk[^"]+"/)) {
            id = substr($0, RSTART + 1, RLENGTH - 2);
            print id;
        }
        in_category = 0;
    }
' "$JAVA_CORE_OPTIONS" | sort -u > "$java_category_ids_file"

awk '
    /fn register_categories\(/ { in_categories = 1; next }
    in_categories && /^}/ { in_categories = 0 }
    in_categories && /\.id\("org\.eclipse\.elk/ {
        if (match($0, /"org\.eclipse\.elk[^"]+"/)) {
            id = substr($0, RSTART + 1, RLENGTH - 2);
            print id;
        }
    }
' "$RUST_CORE_META" | sort -u > "$rust_category_ids_file"

comm -23 "$java_category_ids_file" "$rust_category_ids_file" > "$missing_category_ids_file"
comm -13 "$java_category_ids_file" "$rust_category_ids_file" > "$extra_category_ids_file"

rg -o 'set_category_id\(Some\("[^"]+"\)\)' "$RUST_SOURCES_ROOT" -g '*.rs' \
    | sed -E 's/.*Some\("([^"]+)"\).*/\1/' \
    | sort -u \
    > "$rust_provider_category_ids_file"

grep -Ev '^org\.eclipse\.elk\.' "$rust_provider_category_ids_file" > "$non_qualified_provider_category_ids_file" || true

missing_property_count="$(wc -l < "$missing_property_ids_file" | tr -d ' ')"
extra_property_count="$(wc -l < "$extra_property_ids_file" | tr -d ' ')"
missing_category_count="$(wc -l < "$missing_category_ids_file" | tr -d ' ')"
extra_category_count="$(wc -l < "$extra_category_ids_file" | tr -d ' ')"
non_qualified_provider_category_count="$(wc -l < "$non_qualified_provider_category_ids_file" | tr -d ' ')"

status="ok"
if [ "$missing_property_count" -gt 0 ] \
    || [ "$extra_property_count" -gt 0 ] \
    || [ "$missing_category_count" -gt 0 ] \
    || [ "$extra_category_count" -gt 0 ] \
    || [ "$non_qualified_provider_category_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# CoreOptions Parity"
    echo
    echo "- status: $status"
    echo "- java property ids: $(wc -l < "$java_property_ids_file" | tr -d ' ')"
    echo "- rust property ids: $(wc -l < "$rust_property_ids_file" | tr -d ' ')"
    echo "- missing property ids in rust: $missing_property_count"
    echo "- extra property ids in rust: $extra_property_count"
    echo "- java categories: $(wc -l < "$java_category_ids_file" | tr -d ' ')"
    echo "- rust categories: $(wc -l < "$rust_category_ids_file" | tr -d ' ')"
    echo "- missing categories in rust: $missing_category_count"
    echo "- extra categories in rust: $extra_category_count"
    echo "- non-qualified rust provider category ids: $non_qualified_provider_category_count"
    echo
    echo "## Missing Property IDs"
    if [ "$missing_property_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$missing_property_ids_file"
    fi
    echo
    echo "## Extra Property IDs"
    if [ "$extra_property_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$extra_property_ids_file"
    fi
    echo
    echo "## Missing Categories"
    if [ "$missing_category_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$missing_category_ids_file"
    fi
    echo
    echo "## Extra Categories"
    if [ "$extra_category_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$extra_category_ids_file"
    fi
    echo
    echo "## Non-qualified Provider Category IDs"
    if [ "$non_qualified_provider_category_count" -eq 0 ]; then
        echo "- none"
    else
        sed 's/^/- /' "$non_qualified_provider_category_ids_file"
    fi
} > "$REPORT_FILE"

echo "wrote $REPORT_FILE"

if [ "$status" != "ok" ]; then
    exit 1
fi
