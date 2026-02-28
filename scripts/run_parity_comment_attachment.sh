#!/bin/sh
set -eu

COUNT=${1:-2000}
ITERATIONS=${2:-5}
WARMUP=${3:-1}
OUTPUT=${4:-tests/results_comment_attachment.csv}

cargo run -p org-eclipse-elk-core --bin perf_comment_attachment -- \
  --count "$COUNT" \
  --iterations "$ITERATIONS" \
  --warmup "$WARMUP" \
  --output "$OUTPUT"
