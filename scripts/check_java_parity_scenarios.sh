#!/bin/sh
set -eu

RUST_FILE=${1:-tests/results_layered_issue_scenarios.csv}
JAVA_FILE=${2:-tests/java_results_layered_issue_scenarios.csv}
WINDOW=${3:-3}
THRESHOLDS_FILE=${4:-tests/java_parity_thresholds.csv}

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
if [ ! -f "$THRESHOLDS_FILE" ]; then
  echo "missing thresholds file: $THRESHOLDS_FILE" >&2
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

normalize_threshold_field() {
  value=$1
  trimmed=$(printf "%s" "$value" | awk '{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print $0}')
  if [ -z "$trimmed" ]; then
    echo "0"
  else
    echo "$trimmed"
  fi
}

default_threshold_avg=$(awk -F',' '
  function trim(value) {
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
    return value
  }
  {
    scenario = trim($1)
    if (scenario == "" || tolower(scenario) == "scenario") {
      next
    }
    if (scenario == "*") {
      print trim($2)
      found = 1
      exit
    }
  }
  END {
    if (!found) {
      print 0
    }
  }
' "$THRESHOLDS_FILE")
default_threshold_ops=$(awk -F',' '
  function trim(value) {
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
    return value
  }
  {
    scenario = trim($1)
    if (scenario == "" || tolower(scenario) == "scenario") {
      next
    }
    if (scenario == "*") {
      print trim($3)
      found = 1
      exit
    }
  }
  END {
    if (!found) {
      print 0
    }
  }
' "$THRESHOLDS_FILE")

default_threshold_avg=$(normalize_threshold_field "$default_threshold_avg")
default_threshold_ops=$(normalize_threshold_field "$default_threshold_ops")

threshold_avg_for() {
  scenario=$1
  awk -F',' -v s="$scenario" -v d="$default_threshold_avg" '
    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      return value
    }
    {
      scenario = trim($1)
      if (scenario == "" || tolower(scenario) == "scenario" || scenario == "*") {
        next
      }
      if (scenario == s) {
        value = trim($2)
        if (value == "") {
          print d
        } else {
          print value
        }
        found = 1
        exit
      }
    }
    END {
      if (!found) {
        print d
      }
    }
  ' "$THRESHOLDS_FILE"
}

threshold_ops_for() {
  scenario=$1
  awk -F',' -v s="$scenario" -v d="$default_threshold_ops" '
    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      return value
    }
    {
      scenario = trim($1)
      if (scenario == "" || tolower(scenario) == "scenario" || scenario == "*") {
        next
      }
      if (scenario == s) {
        value = trim($3)
        if (value == "") {
          print d
        } else {
          print value
        }
        found = 1
        exit
      }
    }
    END {
      if (!found) {
        print d
      }
    }
  ' "$THRESHOLDS_FILE"
}

aggregate_file "$RUST_FILE" "$RUST_SCENARIO_COL" "$RUST_AVG_COL" "$RUST_OPS_COL" "$tmp_rust"
aggregate_file "$JAVA_FILE" "$JAVA_SCENARIO_COL" "$JAVA_AVG_COL" "$JAVA_OPS_COL" "$tmp_java"

{
  cut -d',' -f1 "$tmp_rust"
  cut -d',' -f1 "$tmp_java"
} | sort -u > "$tmp_scenarios"

fail=0
compared=0
for scenario in $(cat "$tmp_scenarios"); do
  rust_row=$(awk -F',' -v s="$scenario" '$1 == s { print; exit }' "$tmp_rust")
  java_row=$(awk -F',' -v s="$scenario" '$1 == s { print; exit }' "$tmp_java")
  if [ -z "$rust_row" ] || [ -z "$java_row" ]; then
    echo "skip scenario '$scenario': missing in one side"
    continue
  fi

  compared=$((compared + 1))
  rust_avg=$(echo "$rust_row" | cut -d',' -f2)
  rust_ops=$(echo "$rust_row" | cut -d',' -f3)
  java_avg=$(echo "$java_row" | cut -d',' -f2)
  java_ops=$(echo "$java_row" | cut -d',' -f3)

  avg_regress=$(awk -v rust="$rust_avg" -v java="$java_avg" 'BEGIN { if (java == 0) { print 0 } else { printf "%.2f", ((rust - java) / java) * 100 } }')
  ops_regress=$(awk -v rust="$rust_ops" -v java="$java_ops" 'BEGIN { if (java == 0) { print 0 } else { printf "%.2f", ((java - rust) / java) * 100 } }')

  scenario_threshold_avg=$(threshold_avg_for "$scenario")
  scenario_threshold_ops=$(threshold_ops_for "$scenario")
  scenario_threshold_avg=$(normalize_threshold_field "$scenario_threshold_avg")
  scenario_threshold_ops=$(normalize_threshold_field "$scenario_threshold_ops")

  avg_fail=$(awk -v v="$avg_regress" -v th="$scenario_threshold_avg" 'BEGIN { if (v > th) { print 1 } else { print 0 } }')
  ops_fail=$(awk -v v="$ops_regress" -v th="$scenario_threshold_ops" 'BEGIN { if (v > th) { print 1 } else { print 0 } }')

  if [ "$avg_fail" -eq 1 ]; then
    echo "$scenario: avg_ms regression vs java ${avg_regress}% (> ${scenario_threshold_avg}%)"
    fail=1
  fi
  if [ "$ops_fail" -eq 1 ]; then
    echo "$scenario: scenarios_per_sec regression vs java ${ops_regress}% (> ${scenario_threshold_ops}%)"
    fail=1
  fi
  if [ "$avg_fail" -eq 0 ] && [ "$ops_fail" -eq 0 ]; then
    echo "$scenario: ok vs java (avg_ms Δ${avg_regress}% <= ${scenario_threshold_avg}%, scenarios_per_sec Δ${ops_regress}% <= ${scenario_threshold_ops}%)"
  fi
done

if [ "$compared" -eq 0 ]; then
  echo "no comparable scenarios between rust and java files" >&2
  exit 2
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi
