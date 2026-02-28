#!/bin/sh
set -eu

SOURCE=${1:-tests/results_layered_issue_scenarios.csv}
TARGET=${2:-tests/baselines/layered_issue_scenarios.csv}

if [ ! -f "$SOURCE" ]; then
  echo "missing source parity file: $SOURCE" >&2
  exit 1
fi

mkdir -p "$(dirname "$TARGET")"
cp "$SOURCE" "$TARGET"
echo "updated baseline: $TARGET"
