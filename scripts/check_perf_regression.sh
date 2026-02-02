#!/bin/sh
set -eu

THRESHOLD=${1:-5}
WINDOW=${2:-3}
fail=0

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

check_file() {
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

  avg_delta=$(awk -v prev="$prev_avg" -v curr="$curr_avg" 'BEGIN{ if(prev==0){print 0} else printf "%.2f", ((curr-prev)/prev)*100 }')
  ops_delta=$(awk -v prev="$prev_ops" -v curr="$curr_ops" 'BEGIN{ if(prev==0){print 0} else printf "%.2f", ((prev-curr)/prev)*100 }')

  avg_regress=$(awk -v prev="$prev_avg" -v curr="$curr_avg" -v th="$THRESHOLD" 'BEGIN{ if(prev==0){print 0} else if(((curr-prev)/prev*100) > th){print 1}else{print 0}}')
  ops_regress=$(awk -v prev="$prev_ops" -v curr="$curr_ops" -v th="$THRESHOLD" 'BEGIN{ if(prev==0){print 0} else if(((prev-curr)/prev*100) > th){print 1}else{print 0}}')

  if [ "$avg_regress" -eq 1 ]; then
    echo "$name: avg_ms regression ${avg_delta}% (> ${THRESHOLD}%)"
    fail=1
  fi
  if [ "$ops_regress" -eq 1 ]; then
    echo "$name: ops_per_sec regression ${ops_delta}% (> ${THRESHOLD}%)"
    fail=1
  fi
  if [ "$avg_regress" -eq 0 ] && [ "$ops_regress" -eq 0 ]; then
    echo "$name: ok (avg_ms Δ${avg_delta}%, ops_per_sec Δ${ops_delta}%)"
  fi
}

check_file "comment_attachment" "perf/results_comment_attachment.csv" 6 7
check_file "graph_validation" "perf/results_graph_validation.csv" 8 9
check_file "recursive_layout" "perf/results_recursive_layout.csv" 8 9

if [ "$fail" -ne 0 ]; then
  exit 1
fi
