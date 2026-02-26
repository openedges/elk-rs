#!/bin/sh
set -eu

SOURCE=${1:-parity/java_results_layered_issue_scenarios.csv}
TARGET=${2:-parity/baselines/java_layered_issue_scenarios.csv}

if [ ! -s "$SOURCE" ]; then
  echo "missing or empty Java source baseline file: $SOURCE" >&2
  exit 1
fi

mkdir -p "$(dirname "$TARGET")"
cp "$SOURCE" "$TARGET"
echo "updated Java baseline: $TARGET (from $SOURCE)"
