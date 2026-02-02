#!/bin/sh
set -eu

THRESHOLD=${1:-5}
WINDOW=${2:-3}

sh scripts/run_perf_all.sh
sh scripts/compare_perf_results.sh
sh scripts/summarize_perf_results.sh
sh scripts/check_perf_regression.sh "$THRESHOLD" "$WINDOW"
