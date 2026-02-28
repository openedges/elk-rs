#!/bin/sh
set -eu

NODES=${1:-1000}
EDGES=${2:-2000}
ITERATIONS=${3:-5}
WARMUP=${4:-1}
MODE=${5:-both}
OUTPUT=${6:-tests/results_graph_validation.csv}

cargo run -p org-eclipse-elk-core --bin perf_graph_validation -- \
  --nodes "$NODES" \
  --edges "$EDGES" \
  --iterations "$ITERATIONS" \
  --warmup "$WARMUP" \
  --mode "$MODE" \
  --output "$OUTPUT"
