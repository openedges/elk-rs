#!/bin/sh
set -eu

WINDOW=${1:-1}
MODE=${2:-window}

sh scripts/run_parity_all.sh
PARITY_COMPARE_MODE="$MODE" sh scripts/compare_parity_results.sh "$WINDOW"
sh scripts/summarize_parity_results.sh
