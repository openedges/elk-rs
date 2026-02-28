#!/bin/sh
set -eu

WINDOW=${1:-1}
MODE=${PARITY_COMPARE_MODE:-window}
BASELINE_LAYERED_FILE=${PARITY_BASELINE_LAYERED_FILE:-tests/baselines/layered_issue_scenarios.csv}
BASELINE_RECURSIVE_SCENARIOS_FILE=${PARITY_BASELINE_RECURSIVE_SCENARIOS_FILE:-tests/baselines/recursive_layout_scenarios.csv}

case "$MODE" in
  window|baseline|both) ;;
  *)
    echo "invalid PARITY_COMPARE_MODE: $MODE (expected: window|baseline|both)" >&2
    exit 2
    ;;
esac

mode_enabled() {
  expected=$1
  [ "$MODE" = "$expected" ] || [ "$MODE" = "both" ]
}

avg_column_window() {
  file=$1
  col=$2
  window=$3
  offset=$4

  if [ "$offset" -eq 0 ]; then
    tail -n "$window" "$file" | awk -F',' -v c="$col" '{sum+=$c; n+=1} END{ if(n==0){print 0}else{printf "%.6f", sum/n}}'
  else
    tail -n $((window * (offset + 1))) "$file" | head -n "$window" | awk -F',' -v c="$col" '{sum+=$c; n+=1} END{ if(n==0){print 0}else{printf "%.6f", sum/n}}'
  fi
}

avg_column_all() {
  file=$1
  col=$2
  awk -F',' -v c="$col" '{sum+=$c; n+=1} END{ if(n==0){print 0}else{printf "%.6f", sum/n}}' "$file"
}

latest_config_key_for_scenario() {
  file=$1
  scenario_col=$2
  scenario=$3
  config_col_a=$4
  config_col_b=$5

  if [ "$config_col_a" -le 0 ] || [ "$config_col_b" -le 0 ]; then
    echo ""
    return 0
  fi

  awk -F',' -v sc="$scenario_col" -v s="$scenario" -v ca="$config_col_a" -v cb="$config_col_b" '
    $sc == s { key = $ca "\t" $cb }
    END {
      if (key != "") {
        print key
      }
    }
  ' "$file"
}

compare_two_lines() {
  name=$1
  file=$2
  avg_col=$3
  ops_col=$4

  if [ ! -f "$file" ]; then
    echo "$name: missing $file"
    return 0
  fi

  line_count=$(wc -l < "$file" | tr -d ' ')
  need=$((WINDOW * 2))
  if [ "$line_count" -lt "$need" ]; then
    echo "$name: not enough data (need ${need} lines for window ${WINDOW})"
    return 0
  fi

  prev_avg=$(avg_column_window "$file" "$avg_col" "$WINDOW" 1)
  curr_avg=$(avg_column_window "$file" "$avg_col" "$WINDOW" 0)
  prev_ops=$(avg_column_window "$file" "$ops_col" "$WINDOW" 1)
  curr_ops=$(avg_column_window "$file" "$ops_col" "$WINDOW" 0)

  pct_avg=$(awk -v prev="$prev_avg" -v curr="$curr_avg" 'BEGIN{ if(prev==0){print "n/a"} else printf "%.2f", ((curr-prev)/prev)*100 }')
  pct_ops=$(awk -v prev="$prev_ops" -v curr="$curr_ops" 'BEGIN{ if(prev==0){print "n/a"} else printf "%.2f", ((curr-prev)/prev)*100 }')

  echo "$name: avg_ms=$curr_avg (Δ${pct_avg}%), ops_per_sec=$curr_ops (Δ${pct_ops}%)"
}

compare_per_scenario_window() {
  name=$1
  file=$2
  avg_col=$3
  ops_col=$4
  scenario_col=$5
  config_col_a=${6:-0}
  config_col_b=${7:-0}

  if [ ! -f "$file" ]; then
    echo "$name: missing $file"
    return 0
  fi

  scenarios=$(awk -F',' -v c="$scenario_col" '{print $c}' "$file" | sort -u)
  if [ -z "$scenarios" ]; then
    echo "$name: no scenarios in $file"
    return 0
  fi

  echo "$name:"
  for scenario in $scenarios; do
    tmp_file=$(mktemp)
    latest_config_key=$(latest_config_key_for_scenario "$file" "$scenario_col" "$scenario" "$config_col_a" "$config_col_b")
    if [ -n "$latest_config_key" ]; then
      config_a=$(printf '%s' "$latest_config_key" | awk -F '\t' '{print $1}')
      config_b=$(printf '%s' "$latest_config_key" | awk -F '\t' '{print $2}')
      awk -F',' -v c="$scenario_col" -v s="$scenario" -v ca="$config_col_a" -v cb="$config_col_b" -v a="$config_a" -v b="$config_b" '
        $c == s && $ca == a && $cb == b { print }
      ' "$file" > "$tmp_file"
    else
      awk -F',' -v c="$scenario_col" -v s="$scenario" '$c == s {print}' "$file" > "$tmp_file"
    fi

    line_count=$(wc -l < "$tmp_file" | tr -d ' ')
    need=$((WINDOW * 2))
    if [ "$line_count" -lt "$need" ]; then
      echo "  - $scenario: not enough data (need ${need} lines for window ${WINDOW})"
      rm -f "$tmp_file"
      continue
    fi

    prev_avg=$(avg_column_window "$tmp_file" "$avg_col" "$WINDOW" 1)
    curr_avg=$(avg_column_window "$tmp_file" "$avg_col" "$WINDOW" 0)
    prev_ops=$(avg_column_window "$tmp_file" "$ops_col" "$WINDOW" 1)
    curr_ops=$(avg_column_window "$tmp_file" "$ops_col" "$WINDOW" 0)

    pct_avg=$(awk -v prev="$prev_avg" -v curr="$curr_avg" 'BEGIN{ if(prev==0){print "n/a"} else printf "%.2f", ((curr-prev)/prev)*100 }')
    pct_ops=$(awk -v prev="$prev_ops" -v curr="$curr_ops" 'BEGIN{ if(prev==0){print "n/a"} else printf "%.2f", ((curr-prev)/prev)*100 }')

    echo "  - $scenario: avg_ms=$curr_avg (Δ${pct_avg}%), ops_per_sec=$curr_ops (Δ${pct_ops}%)"
    rm -f "$tmp_file"
  done
}

compare_per_scenario_baseline() {
  name=$1
  current_file=$2
  baseline_file=$3
  avg_col=$4
  ops_col=$5
  scenario_col=$6
  config_col_a=${7:-0}
  config_col_b=${8:-0}

  if [ ! -f "$current_file" ]; then
    echo "$name: missing $current_file"
    return 0
  fi
  if [ ! -f "$baseline_file" ]; then
    echo "$name: missing baseline $baseline_file"
    return 0
  fi

  scenarios=$(awk -F',' -v c="$scenario_col" '{print $c}' "$current_file" | sort -u)
  if [ -z "$scenarios" ]; then
    echo "$name: no scenarios in $current_file"
    return 0
  fi

  echo "$name (baseline: $baseline_file):"
  for scenario in $scenarios; do
    current_tmp=$(mktemp)
    baseline_tmp=$(mktemp)
    latest_config_key=$(latest_config_key_for_scenario "$current_file" "$scenario_col" "$scenario" "$config_col_a" "$config_col_b")
    if [ -n "$latest_config_key" ]; then
      config_a=$(printf '%s' "$latest_config_key" | awk -F '\t' '{print $1}')
      config_b=$(printf '%s' "$latest_config_key" | awk -F '\t' '{print $2}')
      awk -F',' -v c="$scenario_col" -v s="$scenario" -v ca="$config_col_a" -v cb="$config_col_b" -v a="$config_a" -v b="$config_b" '
        $c == s && $ca == a && $cb == b { print }
      ' "$current_file" > "$current_tmp"
    else
      awk -F',' -v c="$scenario_col" -v s="$scenario" '$c == s {print}' "$current_file" > "$current_tmp"
    fi
    awk -F',' -v c="$scenario_col" -v s="$scenario" '$c == s {print}' "$baseline_file" > "$baseline_tmp"

    current_count=$(wc -l < "$current_tmp" | tr -d ' ')
    baseline_count=$(wc -l < "$baseline_tmp" | tr -d ' ')
    if [ "$current_count" -lt "$WINDOW" ]; then
      echo "  - $scenario: not enough current data (need ${WINDOW} lines)"
      rm -f "$current_tmp" "$baseline_tmp"
      continue
    fi
    if [ "$baseline_count" -eq 0 ]; then
      echo "  - $scenario: baseline data missing"
      rm -f "$current_tmp" "$baseline_tmp"
      continue
    fi

    curr_avg=$(avg_column_window "$current_tmp" "$avg_col" "$WINDOW" 0)
    curr_ops=$(avg_column_window "$current_tmp" "$ops_col" "$WINDOW" 0)
    base_avg=$(avg_column_all "$baseline_tmp" "$avg_col")
    base_ops=$(avg_column_all "$baseline_tmp" "$ops_col")

    pct_avg=$(awk -v base="$base_avg" -v curr="$curr_avg" 'BEGIN{ if(base==0){print "n/a"} else printf "%.2f", ((curr-base)/base)*100 }')
    pct_ops=$(awk -v base="$base_ops" -v curr="$curr_ops" 'BEGIN{ if(base==0){print "n/a"} else printf "%.2f", ((curr-base)/base)*100 }')

    echo "  - $scenario: avg_ms=$curr_avg (vs baseline Δ${pct_avg}%), ops_per_sec=$curr_ops (vs baseline Δ${pct_ops}%)"
    rm -f "$current_tmp" "$baseline_tmp"
  done
}

if mode_enabled window; then
  compare_two_lines "comment_attachment" "tests/results_comment_attachment.csv" 6 7
  compare_two_lines "graph_validation" "tests/results_graph_validation.csv" 8 9
  compare_two_lines "recursive_layout" "tests/results_recursive_layout.csv" 8 9
  compare_two_lines "recursive_layout_layered" "tests/results_recursive_layout_layered.csv" 8 9
  compare_per_scenario_window "recursive_layout_scenarios" "tests/results_recursive_layout_scenarios.csv" 9 10 2 6 7
  compare_per_scenario_window "layered_issue_scenarios" "tests/results_layered_issue_scenarios.csv" 6 7 2 3 4
fi

if mode_enabled baseline; then
  compare_per_scenario_baseline \
    "recursive_layout_scenarios" \
    "tests/results_recursive_layout_scenarios.csv" \
    "$BASELINE_RECURSIVE_SCENARIOS_FILE" \
    9 \
    10 \
    2 \
    6 \
    7
  compare_per_scenario_baseline \
    "layered_issue_scenarios" \
    "tests/results_layered_issue_scenarios.csv" \
    "$BASELINE_LAYERED_FILE" \
    6 \
    7 \
    2 \
    3 \
    4
fi
