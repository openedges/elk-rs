#!/bin/sh
set -eu

THRESHOLD=${1:-5}
WINDOW=${2:-3}

cargo test --workspace
cargo clippy --workspace --all-targets
sh scripts/run_perf_and_check.sh "$THRESHOLD" "$WINDOW"
