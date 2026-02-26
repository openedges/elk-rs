#!/bin/sh
set -eu

SOURCE=${1:-parity/results_recursive_layout_scenarios.csv}
TARGET=${2:-parity/baselines/recursive_layout_scenarios.csv}

if [ ! -s "$SOURCE" ]; then
  echo "source file missing or empty: $SOURCE" >&2
  exit 1
fi

mkdir -p "$(dirname "$TARGET")"
cp "$SOURCE" "$TARGET"
echo "updated recursive scenarios baseline: $TARGET (from $SOURCE)"
