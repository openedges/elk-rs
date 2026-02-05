#!/bin/sh
set -eu

NODES=${1:-500}
EDGES=${2:-1000}
ITERATIONS=${3:-5}
WARMUP=${4:-1}
VALIDATE_GRAPH=${5:-false}
VALIDATE_OPTIONS=${6:-false}
OUTPUT=${7:-perf/results_recursive_layout_layered.csv}

sh scripts/run_perf_recursive_layout.sh \
  "$NODES" \
  "$EDGES" \
  "$ITERATIONS" \
  "$WARMUP" \
  layered \
  "$VALIDATE_GRAPH" \
  "$VALIDATE_OPTIONS" \
  "$OUTPUT"
