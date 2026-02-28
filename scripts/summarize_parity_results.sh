#!/bin/sh
set -eu

OUT=${1:-tests/summary.md}

write_section() {
  title=$1
  file=$2
  header=$3
  row_fn=$4
  recent_header=$5
  recent_fn=$6
  recent_count=$7

  {
    echo "## $title"
    if [ ! -s "$file" ]; then
      echo ""
      echo "No data in $file."
      echo ""
      return 0
    fi
    line=$(tail -n 1 "$file")
    echo ""
    printf "%b\n" "$header"
    echo "$row_fn" | sh -s -- "$line"
    echo ""
    if [ -n "$recent_header" ] && [ -n "$recent_fn" ]; then
      echo "Recent runs (last ${recent_count}):"
      echo ""
      printf "%b\n" "$recent_header"
      tail -n "$recent_count" "$file" | while IFS= read -r recent_line; do
        echo "$recent_fn" | sh -s -- "$recent_line"
      done
      echo ""
    fi
  } >> "$OUT"
}

{
  echo "# Parity Summary"
  echo ""
  echo "Generated: $(date -u +\"%Y-%m-%dT%H:%M:%SZ\")"
  echo ""
} > "$OUT"

write_section \
  "Comment Attachment" \
  "tests/results_comment_attachment.csv" \
  "|timestamp|count|iterations|warmup|avg_ms|ops_per_sec|\n|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  count=$(echo "$line" | awk -F"," "{print \$2}")
  iters=$(echo "$line" | awk -F"," "{print \$3}")
  warmup=$(echo "$line" | awk -F"," "{print \$4}")
  avg=$(echo "$line" | awk -F"," "{print \$6}")
  ops=$(echo "$line" | awk -F"," "{print \$7}")
  echo "|$ts|$count|$iters|$warmup|$avg|$ops|"' \
  "|timestamp|count|iterations|warmup|avg_ms|ops_per_sec|\n|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  count=$(echo "$line" | awk -F"," "{print \$2}")
  iters=$(echo "$line" | awk -F"," "{print \$3}")
  warmup=$(echo "$line" | awk -F"," "{print \$4}")
  avg=$(echo "$line" | awk -F"," "{print \$6}")
  ops=$(echo "$line" | awk -F"," "{print \$7}")
  echo "|$ts|$count|$iters|$warmup|$avg|$ops|"' \
  5

write_section \
  "Graph Validation" \
  "tests/results_graph_validation.csv" \
  "|timestamp|mode|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|\n|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  mode=$(echo "$line" | awk -F"," "{print \$2}")
  nodes=$(echo "$line" | awk -F"," "{print \$3}")
  edges=$(echo "$line" | awk -F"," "{print \$4}")
  iters=$(echo "$line" | awk -F"," "{print \$5}")
  warmup=$(echo "$line" | awk -F"," "{print \$6}")
  avg=$(echo "$line" | awk -F"," "{print \$8}")
  ops=$(echo "$line" | awk -F"," "{print \$9}")
  echo "|$ts|$mode|$nodes|$edges|$iters|$warmup|$avg|$ops|"' \
  "|timestamp|mode|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|\n|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  mode=$(echo "$line" | awk -F"," "{print \$2}")
  nodes=$(echo "$line" | awk -F"," "{print \$3}")
  edges=$(echo "$line" | awk -F"," "{print \$4}")
  iters=$(echo "$line" | awk -F"," "{print \$5}")
  warmup=$(echo "$line" | awk -F"," "{print \$6}")
  avg=$(echo "$line" | awk -F"," "{print \$8}")
  ops=$(echo "$line" | awk -F"," "{print \$9}")
  echo "|$ts|$mode|$nodes|$edges|$iters|$warmup|$avg|$ops|"' \
  5

write_section \
  "Recursive Layout" \
  "tests/results_recursive_layout.csv" \
  "|timestamp|algorithm|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|validate_graph|validate_options|\n|---|---|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  algo=$(echo "$line" | awk -F"," "{print \$2}")
  nodes=$(echo "$line" | awk -F"," "{print \$3}")
  edges=$(echo "$line" | awk -F"," "{print \$4}")
  iters=$(echo "$line" | awk -F"," "{print \$5}")
  warmup=$(echo "$line" | awk -F"," "{print \$6}")
  avg=$(echo "$line" | awk -F"," "{print \$8}")
  ops=$(echo "$line" | awk -F"," "{print \$9}")
  vgraph=$(echo "$line" | awk -F"," "{print \$10}")
  vopts=$(echo "$line" | awk -F"," "{print \$11}")
  echo "|$ts|$algo|$nodes|$edges|$iters|$warmup|$avg|$ops|$vgraph|$vopts|"' \
  "|timestamp|algorithm|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|validate_graph|validate_options|\n|---|---|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  algo=$(echo "$line" | awk -F"," "{print \$2}")
  nodes=$(echo "$line" | awk -F"," "{print \$3}")
  edges=$(echo "$line" | awk -F"," "{print \$4}")
  iters=$(echo "$line" | awk -F"," "{print \$5}")
  warmup=$(echo "$line" | awk -F"," "{print \$6}")
  avg=$(echo "$line" | awk -F"," "{print \$8}")
  ops=$(echo "$line" | awk -F"," "{print \$9}")
  vgraph=$(echo "$line" | awk -F"," "{print \$10}")
  vopts=$(echo "$line" | awk -F"," "{print \$11}")
  echo "|$ts|$algo|$nodes|$edges|$iters|$warmup|$avg|$ops|$vgraph|$vopts|"' \
  5

write_section \
  "Recursive Layout (Layered)" \
  "tests/results_recursive_layout_layered.csv" \
  "|timestamp|algorithm|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|validate_graph|validate_options|\n|---|---|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  algo=$(echo "$line" | awk -F"," "{print \$2}")
  nodes=$(echo "$line" | awk -F"," "{print \$3}")
  edges=$(echo "$line" | awk -F"," "{print \$4}")
  iters=$(echo "$line" | awk -F"," "{print \$5}")
  warmup=$(echo "$line" | awk -F"," "{print \$6}")
  avg=$(echo "$line" | awk -F"," "{print \$8}")
  ops=$(echo "$line" | awk -F"," "{print \$9}")
  vgraph=$(echo "$line" | awk -F"," "{print \$10}")
  vopts=$(echo "$line" | awk -F"," "{print \$11}")
  echo "|$ts|$algo|$nodes|$edges|$iters|$warmup|$avg|$ops|$vgraph|$vopts|"' \
  "|timestamp|algorithm|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|validate_graph|validate_options|\n|---|---|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  algo=$(echo "$line" | awk -F"," "{print \$2}")
  nodes=$(echo "$line" | awk -F"," "{print \$3}")
  edges=$(echo "$line" | awk -F"," "{print \$4}")
  iters=$(echo "$line" | awk -F"," "{print \$5}")
  warmup=$(echo "$line" | awk -F"," "{print \$6}")
  avg=$(echo "$line" | awk -F"," "{print \$8}")
  ops=$(echo "$line" | awk -F"," "{print \$9}")
  vgraph=$(echo "$line" | awk -F"," "{print \$10}")
  vopts=$(echo "$line" | awk -F"," "{print \$11}")
  echo "|$ts|$algo|$nodes|$edges|$iters|$warmup|$avg|$ops|$vgraph|$vopts|"' \
  5

write_section \
  "Recursive Layout Scenarios" \
  "tests/results_recursive_layout_scenarios.csv" \
  "|timestamp|scenario|algorithm|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|validate_graph|validate_options|\n|---|---|---|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  scenario=$(echo "$line" | awk -F"," "{print \$2}")
  algo=$(echo "$line" | awk -F"," "{print \$3}")
  nodes=$(echo "$line" | awk -F"," "{print \$4}")
  edges=$(echo "$line" | awk -F"," "{print \$5}")
  iters=$(echo "$line" | awk -F"," "{print \$6}")
  warmup=$(echo "$line" | awk -F"," "{print \$7}")
  avg=$(echo "$line" | awk -F"," "{print \$9}")
  ops=$(echo "$line" | awk -F"," "{print \$10}")
  vgraph=$(echo "$line" | awk -F"," "{print \$11}")
  vopts=$(echo "$line" | awk -F"," "{print \$12}")
  echo "|$ts|$scenario|$algo|$nodes|$edges|$iters|$warmup|$avg|$ops|$vgraph|$vopts|"' \
  "|timestamp|scenario|algorithm|nodes|edges|iterations|warmup|avg_ms|elems_per_sec|validate_graph|validate_options|\n|---|---|---|---|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  scenario=$(echo "$line" | awk -F"," "{print \$2}")
  algo=$(echo "$line" | awk -F"," "{print \$3}")
  nodes=$(echo "$line" | awk -F"," "{print \$4}")
  edges=$(echo "$line" | awk -F"," "{print \$5}")
  iters=$(echo "$line" | awk -F"," "{print \$6}")
  warmup=$(echo "$line" | awk -F"," "{print \$7}")
  avg=$(echo "$line" | awk -F"," "{print \$9}")
  ops=$(echo "$line" | awk -F"," "{print \$10}")
  vgraph=$(echo "$line" | awk -F"," "{print \$11}")
  vopts=$(echo "$line" | awk -F"," "{print \$12}")
  echo "|$ts|$scenario|$algo|$nodes|$edges|$iters|$warmup|$avg|$ops|$vgraph|$vopts|"' \
  10

write_section \
  "Layered Issue Scenarios" \
  "tests/results_layered_issue_scenarios.csv" \
  "|timestamp|scenario|iterations|warmup|elapsed_nanos|avg_ms|scenarios_per_sec|\n|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  scenario=$(echo "$line" | awk -F"," "{print \$2}")
  iters=$(echo "$line" | awk -F"," "{print \$3}")
  warmup=$(echo "$line" | awk -F"," "{print \$4}")
  nanos=$(echo "$line" | awk -F"," "{print \$5}")
  avg=$(echo "$line" | awk -F"," "{print \$6}")
  throughput=$(echo "$line" | awk -F"," "{print \$7}")
  echo "|$ts|$scenario|$iters|$warmup|$nanos|$avg|$throughput|"' \
  "|timestamp|scenario|iterations|warmup|elapsed_nanos|avg_ms|scenarios_per_sec|\n|---|---|---|---|---|---|---|" \
  'line=$1
  ts=$(echo "$line" | awk -F"," "{print \$1}")
  scenario=$(echo "$line" | awk -F"," "{print \$2}")
  iters=$(echo "$line" | awk -F"," "{print \$3}")
  warmup=$(echo "$line" | awk -F"," "{print \$4}")
  nanos=$(echo "$line" | awk -F"," "{print \$5}")
  avg=$(echo "$line" | awk -F"," "{print \$6}")
  throughput=$(echo "$line" | awk -F"," "{print \$7}")
  echo "|$ts|$scenario|$iters|$warmup|$nanos|$avg|$throughput|"' \
  10

echo "Wrote $OUT"
