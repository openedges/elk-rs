#!/bin/sh
set -eu

WINDOW=${1:-1}
MODE=${2:-window}

sh scripts/run_perf_all.sh
PERF_COMPARE_MODE="$MODE" sh scripts/compare_perf_results.sh "$WINDOW"
sh scripts/summarize_perf_results.sh
