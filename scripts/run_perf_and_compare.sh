#!/bin/sh
set -eu

WINDOW=${1:-1}

sh scripts/run_perf_all.sh
sh scripts/compare_perf_results.sh "$WINDOW"
sh scripts/summarize_perf_results.sh
