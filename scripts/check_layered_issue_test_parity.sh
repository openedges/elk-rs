#!/usr/bin/env sh
set -eu

JAVA_ISSUE_TEST_ROOT="${JAVA_ISSUE_TEST_ROOT:-external/elk/test/org.eclipse.elk.alg.layered.test/src/org/eclipse/elk/alg/layered/issues}"
RUST_ISSUE_TEST_ROOT="${RUST_ISSUE_TEST_ROOT:-plugins/org.eclipse.elk.alg.layered/tests}"
REPORT_FILE="${1:-parity/layered_issue_test_parity.md}"
STRICT_MODE="${LAYERED_ISSUE_TEST_PARITY_STRICT:-false}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/layered-issue-test-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

mkdir -p "$(dirname "$REPORT_FILE")"

java_rows="$tmp_dir/java_rows.tsv"
rust_rows="$tmp_dir/rust_rows.tsv"
expected_rust="$tmp_dir/expected_rust.txt"
actual_rust="$tmp_dir/actual_rust.txt"
missing_rust="$tmp_dir/missing_rust.txt"
extra_rust="$tmp_dir/extra_rust.txt"
drift_rows="$tmp_dir/drift_rows.tsv"

touch "$java_rows" "$rust_rows" "$expected_rust" "$actual_rust" "$drift_rows"

(rg --files "$JAVA_ISSUE_TEST_ROOT" -g '*Issue*Test.java' || true) \
    | sort \
    | while IFS= read -r java_file; do
        [ -f "$java_file" ] || continue
        java_base="$(basename "$java_file")"
        issue_name="$(printf '%s' "$java_base" | sed -E 's/^Issue//; s/Test\.java$//')"
        rust_issue_name="$(printf '%s' "$issue_name" \
            | sed -E 's/([a-z0-9])([A-Z])/\1_\2/g; s/([A-Za-z])([0-9])/\1_\2/g' \
            | tr 'A-Z' 'a-z')"
        rust_base="issue_${rust_issue_name}_test.rs"
        java_count="$(rg -n '@Test|@TestAfterProcessor\(' "$java_file" | wc -l | tr -d ' ')"
        printf '%s\t%s\t%s\t%s\n' "$issue_name" "$java_base" "$rust_base" "$java_count" >> "$java_rows"
        printf '%s\n' "$rust_base" >> "$expected_rust"
    done

(rg --files "$RUST_ISSUE_TEST_ROOT" -g 'issue_*_test.rs' || true) \
    | sort \
    | while IFS= read -r rust_file; do
        [ -f "$rust_file" ] || continue
        rust_base="$(basename "$rust_file")"
        rust_count="$(rg -n '#\[test\]' "$rust_file" | wc -l | tr -d ' ')"
        printf '%s\t%s\n' "$rust_base" "$rust_count" >> "$rust_rows"
        printf '%s\n' "$rust_base" >> "$actual_rust"
    done

sort -u "$expected_rust" -o "$expected_rust"
sort -u "$actual_rust" -o "$actual_rust"
comm -23 "$expected_rust" "$actual_rust" > "$missing_rust"
comm -13 "$expected_rust" "$actual_rust" > "$extra_rust"

{
    while IFS="$(printf '\t')" read -r issue_name java_base rust_base java_count; do
        rust_count="$(awk -F '\t' -v file="$rust_base" '$1 == file { print $2 }' "$rust_rows")"
        if [ -z "$rust_count" ]; then
            rust_count=0
        fi
        status="ok"
        if [ "$java_count" -ne "$rust_count" ]; then
            status="drift"
        fi
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$issue_name" "$java_base" "$rust_base" "$java_count" "$rust_count" "$status"
    done < "$java_rows"
} | sort -V > "$drift_rows"

java_issue_files="$(wc -l < "$java_rows" | tr -d ' ')"
rust_issue_files="$(wc -l < "$rust_rows" | tr -d ' ')"
missing_count="$(wc -l < "$missing_rust" | tr -d ' ')"
extra_count="$(wc -l < "$extra_rust" | tr -d ' ')"
drift_count="$(awk -F '\t' '$6 == "drift" {count += 1} END {print count + 0}' "$drift_rows")"
java_test_total="$(awk -F '\t' '{sum += $4} END {print sum + 0}' "$drift_rows")"
rust_test_total="$(awk -F '\t' '{sum += $5} END {print sum + 0}' "$drift_rows")"

status="ok"
if [ "$missing_count" -gt 0 ] || [ "$extra_count" -gt 0 ] || [ "$drift_count" -gt 0 ]; then
    status="drift"
fi

{
    echo "# Layered Issue Test Parity"
    echo
    echo "- status: $status"
    echo "- java issue files: $java_issue_files"
    echo "- rust issue files: $rust_issue_files"
    echo "- java @Test total: $java_test_total"
    echo "- rust #[test] total: $rust_test_total"
    echo "- count drift files: $drift_count"
    echo "- missing rust files: $missing_count"
    echo "- extra rust files: $extra_count"
    echo
    echo "| issue | java_file | rust_file | java_tests | rust_tests | status |"
    echo "| --- | --- | --- | ---: | ---: | --- |"
    awk -F '\t' '
        {
            printf("| %s | %s | %s | %s | %s | %s |\n", $1, $2, $3, $4, $5, $6)
        }
    ' "$drift_rows"
    if [ "$missing_count" -gt 0 ]; then
        echo
        echo "## Missing Rust Files"
        echo
        while IFS= read -r file; do
            [ -n "$file" ] || continue
            echo "- $file"
        done < "$missing_rust"
    fi
    if [ "$extra_count" -gt 0 ]; then
        echo
        echo "## Extra Rust Files"
        echo
        while IFS= read -r file; do
            [ -n "$file" ] || continue
            echo "- $file"
        done < "$extra_rust"
    fi
} > "$REPORT_FILE"

echo "wrote layered issue parity report: $REPORT_FILE"

if [ "$STRICT_MODE" = "true" ] && [ "$status" != "ok" ]; then
    exit 1
fi
