#!/bin/sh
set -eu

THRESHOLD=${1:-5}
WINDOW=${2:-3}
MODE=${3:-window}

sh scripts/run_parity_all.sh
PARITY_COMPARE_MODE="$MODE" sh scripts/compare_parity_results.sh "$WINDOW"
sh scripts/summarize_parity_results.sh
PARITY_COMPARE_MODE="$MODE" sh scripts/check_parity_regression.sh "$THRESHOLD" "$WINDOW"
