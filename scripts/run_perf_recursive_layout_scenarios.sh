#!/bin/sh
set -eu

SCENARIOS_INPUT=${1:-}
ITERATIONS=${2:-5}
WARMUP=${3:-1}
OUTPUT=${4:-perf/results_recursive_layout_scenarios.csv}
SCENARIO_PROFILE=${PERF_RECURSIVE_SCENARIO_PROFILE:-default}

if [ -n "$SCENARIOS_INPUT" ]; then
  SCENARIOS=$SCENARIOS_INPUT
else
  case "$SCENARIO_PROFILE" in
    quick)
      SCENARIOS=fixed_dense,random_sparse,box_validated
      ;;
    default)
      SCENARIOS=fixed_dense,fixed_sparse,random_dense,random_sparse,box_sparse,fixed_validated,random_validated,box_validated
      ;;
    full)
      SCENARIOS=fixed_dense,fixed_sparse,random_dense,random_sparse,box_sparse,fixed_validated,random_validated,box_validated,box_large
      ;;
    *)
      echo "unknown recursive scenario profile: $SCENARIO_PROFILE (expected quick|default|full)" >&2
      exit 2
      ;;
  esac
fi

mkdir -p "$(dirname "$OUTPUT")"

IFS=',' 
for scenario in $SCENARIOS; do
  scenario_trimmed=$(echo "$scenario" | awk '{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print $0}')
  if [ -z "$scenario_trimmed" ]; then
    continue
  fi

  algorithm=
  nodes=
  edges=
  validate_graph=false
  validate_options=false

  case "$scenario_trimmed" in
    fixed_dense)
      algorithm=fixed
      nodes=500
      edges=1000
      ;;
    fixed_sparse)
      algorithm=fixed
      nodes=500
      edges=200
      ;;
    random_dense)
      algorithm=random
      nodes=500
      edges=1000
      ;;
    random_sparse)
      algorithm=random
      nodes=500
      edges=200
      ;;
    box_sparse)
      algorithm=box
      nodes=500
      edges=0
      ;;
    box_large)
      algorithm=box
      nodes=1500
      edges=0
      ;;
    fixed_validated)
      algorithm=fixed
      nodes=500
      edges=1000
      validate_graph=true
      validate_options=true
      ;;
    random_validated)
      algorithm=random
      nodes=500
      edges=1000
      validate_graph=true
      validate_options=true
      ;;
    box_validated)
      algorithm=box
      nodes=500
      edges=0
      validate_graph=true
      validate_options=true
      ;;
    *)
      echo "unknown recursive layout scenario: $scenario_trimmed" >&2
      exit 2
      ;;
  esac

  tmp_output=$(mktemp)
  trap 'rm -f "$tmp_output"' EXIT HUP INT TERM

  sh scripts/run_perf_recursive_layout.sh \
    "$nodes" \
    "$edges" \
    "$ITERATIONS" \
    "$WARMUP" \
    "$algorithm" \
    "$validate_graph" \
    "$validate_options" \
    "$tmp_output"

  awk -F',' -v s="$scenario_trimmed" '
    NF >= 11 {
      printf "%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n",
        $1, s, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
    }
  ' "$tmp_output" >> "$OUTPUT"

  rm -f "$tmp_output"
  trap - EXIT HUP INT TERM
done
