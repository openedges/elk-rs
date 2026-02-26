#!/bin/sh
set -eu

NODES=${1:-500}
EDGES=${2:-1000}
ITERATIONS=${3:-5}
WARMUP=${4:-1}
ALGORITHM=${5:-fixed}
VALIDATE_GRAPH=${6:-false}
VALIDATE_OPTIONS=${7:-false}
OUTPUT=${8:-parity/results_recursive_layout.csv}

cargo run -p org-eclipse-elk-core --bin perf_recursive_layout -- \
  --nodes "$NODES" \
  --edges "$EDGES" \
  --iterations "$ITERATIONS" \
  --warmup "$WARMUP" \
  --algorithm "$ALGORITHM" \
  --validate-graph "$VALIDATE_GRAPH" \
  --validate-options "$VALIDATE_OPTIONS" \
  --output "$OUTPUT"
