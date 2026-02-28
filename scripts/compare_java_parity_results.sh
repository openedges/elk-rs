#!/bin/sh
set -eu

RUST_FILE=${1:-tests/results_layered_issue_scenarios.csv}
JAVA_FILE=${2:-tests/java_results_layered_issue_scenarios.csv}
WINDOW=${3:-3}
OUTPUT=${4:-tests/java_vs_rust_layered_issue.md}

RUST_SCENARIO_COL=${RUST_SCENARIO_COL:-2}
RUST_AVG_COL=${RUST_AVG_COL:-6}
RUST_OPS_COL=${RUST_OPS_COL:-7}
JAVA_SCENARIO_COL=${JAVA_SCENARIO_COL:-2}
JAVA_AVG_COL=${JAVA_AVG_COL:-6}
JAVA_OPS_COL=${JAVA_OPS_COL:-7}

if [ ! -f "$RUST_FILE" ]; then
  echo "missing rust parity file: $RUST_FILE" >&2
  exit 1
fi
if [ ! -f "$JAVA_FILE" ]; then
  echo "missing java parity file: $JAVA_FILE" >&2
  exit 1
fi

tmp_rust=$(mktemp)
tmp_java=$(mktemp)
tmp_scenarios=$(mktemp)
trap 'rm -f "$tmp_rust" "$tmp_java" "$tmp_scenarios"' EXIT HUP INT TERM

aggregate_file() {
  file=$1
  scenario_col=$2
  avg_col=$3
  ops_col=$4
  out_file=$5

  awk -F',' -v sc="$scenario_col" -v ac="$avg_col" -v oc="$ops_col" -v w="$WINDOW" '
  function trim(value) {
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
    return value
  }
  function is_number(value) {
    return value ~ /^-?[0-9]+([.][0-9]+)?([eE][-+]?[0-9]+)?$/
  }
  {
    s = trim($sc)
    a = trim($ac)
    o = trim($oc)
    if (s == "" || tolower(s) == "scenario") {
      next
    }
    if (!is_number(a) || !is_number(o)) {
      next
    }
    n[s] += 1
    avg[s, n[s]] = a + 0
    ops[s, n[s]] = o + 0
  }
  END {
    for (s in n) {
      start = n[s] - w + 1
      if (start < 1) {
        start = 1
      }
      sum_avg = 0
      sum_ops = 0
      count = 0
      for (i = start; i <= n[s]; i++) {
        sum_avg += avg[s, i]
        sum_ops += ops[s, i]
        count += 1
      }
      if (count > 0) {
        printf "%s,%.6f,%.6f\n", s, sum_avg / count, sum_ops / count
      }
    }
  }' "$file" | sort > "$out_file"
}

aggregate_file "$RUST_FILE" "$RUST_SCENARIO_COL" "$RUST_AVG_COL" "$RUST_OPS_COL" "$tmp_rust"
aggregate_file "$JAVA_FILE" "$JAVA_SCENARIO_COL" "$JAVA_AVG_COL" "$JAVA_OPS_COL" "$tmp_java"

{
  cut -d',' -f1 "$tmp_rust"
  cut -d',' -f1 "$tmp_java"
} | sort -u > "$tmp_scenarios"

better_count=0
slower_count=0
common_count=0

mkdir -p "$(dirname "$OUTPUT")"
{
  echo "# Java vs Rust Layered Issue Perf"
  echo
  echo "- rust file: \`$RUST_FILE\`"
  echo "- java file: \`$JAVA_FILE\`"
  echo "- window: \`$WINDOW\`"
  echo
  echo "| scenario | rust_avg_ms | java_avg_ms | avg_delta_vs_java_% | rust_scenarios_per_sec | java_scenarios_per_sec | ops_delta_vs_java_% |"
  echo "|---|---:|---:|---:|---:|---:|---:|"

  while IFS= read -r scenario; do
    if [ -z "$scenario" ]; then
      continue
    fi

    rust_row=$(awk -F',' -v s="$scenario" '$1 == s { print; exit }' "$tmp_rust")
    java_row=$(awk -F',' -v s="$scenario" '$1 == s { print; exit }' "$tmp_java")

    if [ -z "$rust_row" ] || [ -z "$java_row" ]; then
      rust_avg="n/a"
      rust_ops="n/a"
      java_avg="n/a"
      java_ops="n/a"
      avg_delta="n/a"
      ops_delta="n/a"
    else
      common_count=$((common_count + 1))
      rust_avg=$(echo "$rust_row" | cut -d',' -f2)
      rust_ops=$(echo "$rust_row" | cut -d',' -f3)
      java_avg=$(echo "$java_row" | cut -d',' -f2)
      java_ops=$(echo "$java_row" | cut -d',' -f3)

      avg_delta=$(awk -v rust="$rust_avg" -v java="$java_avg" 'BEGIN { if (java == 0) { print "n/a" } else { printf "%.2f", ((rust - java) / java) * 100 } }')
      ops_delta=$(awk -v rust="$rust_ops" -v java="$java_ops" 'BEGIN { if (java == 0) { print "n/a" } else { printf "%.2f", ((rust - java) / java) * 100 } }')

      is_better=$(awk -v avg="$avg_delta" -v ops="$ops_delta" 'BEGIN { if (avg != "n/a" && ops != "n/a" && avg <= 0 && ops >= 0) { print 1 } else { print 0 } }')
      is_slower=$(awk -v avg="$avg_delta" -v ops="$ops_delta" 'BEGIN { if (avg != "n/a" && ops != "n/a" && avg > 0 && ops < 0) { print 1 } else { print 0 } }')
      better_count=$((better_count + is_better))
      slower_count=$((slower_count + is_slower))
    fi

    echo "| $scenario | $rust_avg | $java_avg | $avg_delta | $rust_ops | $java_ops | $ops_delta |"
  done < "$tmp_scenarios"

  echo
  echo "- common scenarios: $common_count"
  echo "- rust better/equal on both metrics: $better_count"
  echo "- rust slower on both metrics: $slower_count"
} > "$OUTPUT"

echo "wrote $OUTPUT"
