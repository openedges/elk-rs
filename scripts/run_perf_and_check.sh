#!/bin/sh
set -eu

THRESHOLD=${1:-5}
WINDOW=${2:-3}
MODE=${3:-window}

sh scripts/run_perf_all.sh
PERF_COMPARE_MODE="$MODE" sh scripts/compare_perf_results.sh "$WINDOW"
sh scripts/summarize_perf_results.sh
PERF_COMPARE_MODE="$MODE" sh scripts/check_perf_regression.sh "$THRESHOLD" "$WINDOW"
