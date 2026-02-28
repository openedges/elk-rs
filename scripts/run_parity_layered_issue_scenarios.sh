#!/bin/sh
set -eu

SCENARIOS=${1:-issue_405,issue_603,issue_680,issue_871,issue_905}
ITERATIONS=${2:-20}
WARMUP=${3:-3}
OUTPUT=${4:-tests/results_layered_issue_scenarios.csv}

cargo run -p org-eclipse-elk-alg-layered --bin perf_layered_issue_scenarios -- \
  --scenarios "$SCENARIOS" \
  --iterations "$ITERATIONS" \
  --warmup "$WARMUP" \
  --output "$OUTPUT"
