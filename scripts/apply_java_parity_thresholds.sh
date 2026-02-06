#!/usr/bin/env sh
set -eu

SOURCE="${1:-perf/java_parity_thresholds.suggested.csv}"
TARGET="${2:-perf/java_parity_thresholds.csv}"
REQUIRED_SCENARIOS="${JAVA_PARITY_REQUIRED_SCENARIOS:-issue_405,issue_603,issue_680,issue_871,issue_905}"

if [ ! -f "$SOURCE" ]; then
  echo "missing source thresholds file: $SOURCE" >&2
  exit 1
fi

header="$(head -n 1 "$SOURCE" | tr -d '\r')"
expected_header="scenario,max_avg_ms_regression_pct,max_scenarios_per_sec_regression_pct"
if [ "$header" != "$expected_header" ]; then
  echo "invalid thresholds header in $SOURCE" >&2
  echo "expected: $expected_header" >&2
  echo "actual:   $header" >&2
  exit 2
fi

tmp_source_norm="$(mktemp "${TMPDIR:-/tmp}/java-parity-thresholds.source.XXXXXX")"
trap 'rm -f "$tmp_source_norm"' EXIT INT TERM
awk 'NF > 0 { print $0 }' "$SOURCE" > "$tmp_source_norm"

if ! awk -F',' '
  function trim(v) {
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
    return v
  }
  {
    scenario = trim($1)
    if (tolower(scenario) == "scenario" || scenario == "") {
      next
    }
    if (scenario == "*") {
      found = 1
      exit
    }
  }
  END { exit(found ? 0 : 1) }
' "$tmp_source_norm"; then
  echo "thresholds file must include wildcard row '*'" >&2
  exit 2
fi

IFS=','
for scenario in $REQUIRED_SCENARIOS; do
  scenario="$(printf '%s' "$scenario" | awk '{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print $0}')"
  [ -n "$scenario" ] || continue
  if ! awk -F',' -v s="$scenario" '
    function trim(v) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
      return v
    }
    {
      scenario = trim($1)
      if (tolower(scenario) == "scenario" || scenario == "") {
        next
      }
      if (scenario == s) {
        found = 1
        exit
      }
    }
    END { exit(found ? 0 : 1) }
  ' "$tmp_source_norm"; then
    echo "missing required scenario in thresholds file: $scenario" >&2
    exit 2
  fi
done
unset IFS

mkdir -p "$(dirname "$TARGET")"
cp "$tmp_source_norm" "$TARGET"
echo "updated java parity thresholds: $TARGET (from $SOURCE)"
