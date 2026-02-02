#!/bin/sh
set -eu

WINDOW=${1:-1}

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

compare_two_lines "comment_attachment" "perf/results_comment_attachment.csv" 6 7
compare_two_lines "graph_validation" "perf/results_graph_validation.csv" 8 9
compare_two_lines "recursive_layout" "perf/results_recursive_layout.csv" 8 9
