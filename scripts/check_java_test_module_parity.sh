#!/usr/bin/env sh
set -eu

JAVA_TEST_ROOT="${JAVA_TEST_ROOT:-external/elk/test}"
RUST_PLUGIN_ROOT="${RUST_PLUGIN_ROOT:-plugins}"
REPORT_FILE="${1:-parity/java_test_module_parity.md}"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/java-test-module-parity.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

rows_tsv="$tmp_dir/rows.tsv"
mapped_rust_paths="$tmp_dir/mapped_rust_paths.txt"
all_rust_test_modules="$tmp_dir/all_rust_test_modules.txt"
rust_only_modules="$tmp_dir/rust_only_modules.txt"

mkdir -p "$(dirname "$REPORT_FILE")"
: > "$rows_tsv"
: > "$mapped_rust_paths"

cat <<'EOF' > "$tmp_dir/mappings.tsv"
org.eclipse.elk.alg.common.test	plugins/org.eclipse.elk.alg.common	direct	alg.common
org.eclipse.elk.alg.disco.test	plugins/org.eclipse.elk.alg.disco	direct	alg.disco
org.eclipse.elk.alg.force.test	plugins/org.eclipse.elk.alg.force	direct	alg.force
org.eclipse.elk.alg.layered.test	plugins/org.eclipse.elk.alg.layered	direct	alg.layered
org.eclipse.elk.alg.mrtree.test	plugins/org.eclipse.elk.alg.mrtree	direct	alg.mrtree
org.eclipse.elk.alg.radial.test	plugins/org.eclipse.elk.alg.radial	direct	alg.radial
org.eclipse.elk.alg.rectpacking.test	plugins/org.eclipse.elk.alg.rectpacking	direct	alg.rectpacking
org.eclipse.elk.alg.spore.test	plugins/org.eclipse.elk.alg.spore	direct	alg.spore
org.eclipse.elk.alg.topdown.test	plugins/org.eclipse.elk.alg.topdownpacking	direct	alg.topdown -> topdownpacking
org.eclipse.elk.core.test	plugins/org.eclipse.elk.core	direct	core
org.eclipse.elk.graph.json.test	plugins/org.eclipse.elk.graph.json	direct	graph.json
org.eclipse.elk.graph.test	plugins/org.eclipse.elk.graph	direct	graph
org.eclipse.elk.alg.test	n/a	no_direct	java integration harness (no 1:1 rust crate)
org.eclipse.elk.shared.test	n/a	no_direct	java shared test utilities (no 1:1 rust crate)
EOF

while IFS="$(printf '\t')" read -r java_module rust_path mapping_type note; do
    java_dir="$JAVA_TEST_ROOT/$java_module/src"
    java_classes=0
    java_tests=0
    if [ -d "$java_dir" ]; then
        java_classes="$(rg --files "$java_dir" -g '*.java' | wc -l | tr -d ' ')"
        java_tests="$(rg -n '@Test|@TestAfterProcessor\(' "$java_dir" -g '*.java' | wc -l | tr -d ' ')"
    fi

    rust_test_files="n/a"
    rust_tests="n/a"
    delta="n/a"

    if [ "$mapping_type" = "direct" ]; then
        rust_test_files=0
        rust_tests=0
        if [ -n "$rust_path" ] && [ -d "$rust_path" ]; then
            rust_test_files="$(rg -l '#\[test\]' "$rust_path" -g '*.rs' | wc -l | tr -d ' ')"
            rust_tests="$(rg -n '#\[test\]' "$rust_path" -g '*.rs' | wc -l | tr -d ' ')"
            printf '%s\n' "$rust_path" >> "$mapped_rust_paths"
        fi
        delta=$((rust_tests - java_tests))
    fi

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$java_module" "$rust_path" "$mapping_type" "$java_classes" "$java_tests" \
        "$rust_test_files" "$rust_tests" "$delta" >> "$rows_tsv"
done < "$tmp_dir/mappings.tsv"

for d in "$RUST_PLUGIN_ROOT"/org.eclipse.elk*; do
    [ -d "$d" ] || continue
    if rg -q '#\[test\]' "$d" -g '*.rs'; then
        printf '%s\n' "$d" >> "$all_rust_test_modules"
    fi
done

sort -u "$mapped_rust_paths" -o "$mapped_rust_paths"
sort -u "$all_rust_test_modules" -o "$all_rust_test_modules"
comm -23 "$all_rust_test_modules" "$mapped_rust_paths" > "$rust_only_modules"

java_total_modules="$(wc -l < "$rows_tsv" | tr -d ' ')"
java_total_tests="$(awk -F '\t' '{sum += $5} END {print sum + 0}' "$rows_tsv")"
java_total_classes="$(awk -F '\t' '{sum += $4} END {print sum + 0}' "$rows_tsv")"

direct_java_modules="$(awk -F '\t' '$3 == "direct" {count += 1} END {print count + 0}' "$rows_tsv")"
direct_java_tests="$(awk -F '\t' '$3 == "direct" {sum += $5} END {print sum + 0}' "$rows_tsv")"
direct_rust_tests="$(awk -F '\t' '$3 == "direct" {sum += $7} END {print sum + 0}' "$rows_tsv")"
direct_delta=$((direct_rust_tests - direct_java_tests))

no_direct_java_tests="$(awk -F '\t' '$3 == "no_direct" {sum += $5} END {print sum + 0}' "$rows_tsv")"

all_rust_tests="$(rg -n '#\[test\]' "$RUST_PLUGIN_ROOT" -g '*.rs' | wc -l | tr -d ' ')"
all_rust_test_modules_count="$(wc -l < "$all_rust_test_modules" | tr -d ' ')"
rust_only_modules_count="$(wc -l < "$rust_only_modules" | tr -d ' ')"

layered_issue_status="unknown"
layered_issue_java_tests="unknown"
layered_issue_rust_tests="unknown"
if [ -f "parity/layered_issue_test_parity.md" ]; then
    layered_issue_status="$(awk -F': ' '/^- status:/ {print $2; exit}' parity/layered_issue_test_parity.md)"
    layered_issue_java_tests="$(awk -F': ' '/^- java @Test total:/ {print $2; exit}' parity/layered_issue_test_parity.md)"
    layered_issue_rust_tests="$(awk -F': ' '/^- rust #\[test\] total:/ {print $2; exit}' parity/layered_issue_test_parity.md)"
fi

{
    echo "# Java Test Module Parity Matrix"
    echo
    echo "- java modules scanned: $java_total_modules"
    echo "- java test classes total: $java_total_classes"
    echo "- java test methods total (\`@Test\` + \`@TestAfterProcessor\`): $java_total_tests"
    echo "- rust test methods total (\`#[test]\` in plugins): $all_rust_tests"
    echo "- direct-mapped java modules: $direct_java_modules"
    echo "- direct-mapped java tests: $direct_java_tests"
    echo "- direct-mapped rust tests: $direct_rust_tests"
    echo "- direct-mapped delta (rust - java): $direct_delta"
    echo "- java tests in no-direct modules: $no_direct_java_tests"
    echo "- rust modules with tests: $all_rust_test_modules_count"
    echo "- rust-only test modules (not in direct map): $rust_only_modules_count"
    echo "- layered issue parity snapshot: status=$layered_issue_status, java=$layered_issue_java_tests, rust=$layered_issue_rust_tests"
    echo
    echo "| java_module | rust_target | mapping | java_classes | java_tests | rust_test_files | rust_tests | delta_rust_minus_java |"
    echo "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |"
    awk -F '\t' '
        {
            rust_target = $2
            printf("| %s | %s | %s | %s | %s | %s | %s | %s |\n", $1, rust_target, $3, $4, $5, $6, $7, $8)
        }
    ' "$rows_tsv"
    echo
    echo "## Notes"
    echo
    echo '- `mapping=direct` rows are crate-level structural mapping, not method-level 1:1 semantics proof.'
    echo '- `org.eclipse.elk.alg.test` and `org.eclipse.elk.shared.test` are treated as no-direct due to architecture mismatch.'
    echo '- For layered issue method-level parity, use `parity/layered_issue_test_parity.md`.'
    echo
    if [ "$rust_only_modules_count" -gt 0 ]; then
        echo "## Rust-only Test Modules"
        echo
        while IFS= read -r module; do
            [ -n "$module" ] || continue
            echo "- $module"
        done < "$rust_only_modules"
    fi
} > "$REPORT_FILE"

echo "wrote java test module parity report: $REPORT_FILE"
