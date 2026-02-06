#!/bin/sh
set -eu

SOURCE=${1:-perf/results_layered_issue_scenarios.csv}
TARGET=${2:-perf/baselines/layered_issue_scenarios.csv}

if [ ! -f "$SOURCE" ]; then
  echo "missing source perf file: $SOURCE" >&2
  exit 1
fi

mkdir -p "$(dirname "$TARGET")"
cp "$SOURCE" "$TARGET"
echo "updated baseline: $TARGET"
