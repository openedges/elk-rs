#!/usr/bin/env sh
set -eu

RESULTS_FILE="${1:-perf/results_recursive_layout_scenarios.csv}"
PROFILE="${2:-${PERF_RECURSIVE_SCENARIO_PROFILE:-default}}"
REPORT_FILE="${3:-perf/recursive_runtime_budget.md}"
STRICT_MODE="${RECURSIVE_RUNTIME_BUDGET_STRICT:-true}"

case "$PROFILE" in
  quick)
    BUDGET_MS="${RECURSIVE_BUDGET_MS_QUICK:-40}"
    ;;
  default)
    BUDGET_MS="${RECURSIVE_BUDGET_MS_DEFAULT:-60}"
    ;;
  full)
    BUDGET_MS="${RECURSIVE_BUDGET_MS_FULL:-120}"
    ;;
  *)
    echo "unknown recursive runtime budget profile: $PROFILE (expected quick|default|full)" >&2
    exit 2
    ;;
esac

if ! printf "%s\n" "$BUDGET_MS" | awk 'BEGIN{ok=0} /^[0-9]+([.][0-9]+)?$/ {ok=1} END{exit ok?0:1}'; then
  echo "invalid recursive runtime budget value: $BUDGET_MS" >&2
  exit 2
fi

mkdir -p "$(dirname "$REPORT_FILE")"

if [ ! -f "$RESULTS_FILE" ]; then
  {
    echo "# Recursive Runtime Budget"
    echo
    echo "- status: missing_results"
    echo "- results_file: $RESULTS_FILE"
    echo "- profile: $PROFILE"
    echo "- budget_ms: $BUDGET_MS"
    echo "- scenarios_checked: 0"
    echo "- violations: 0"
    echo
    echo "## Violations (scenario | avg_ms | budget_ms)"
    echo "- none"
  } > "$REPORT_FILE"
  if [ "$STRICT_MODE" = "true" ]; then
    echo "recursive runtime budget check failed: results file missing ($RESULTS_FILE)" >&2
    exit 1
  fi
  echo "wrote recursive runtime budget report: $REPORT_FILE"
  exit 0
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/recursive-runtime-budget.XXXXXX")"
cleanup_tmp() {
  rm -rf "$tmp_dir"
}
trap cleanup_tmp EXIT INT TERM

latest_file="$tmp_dir/latest.tsv"
violations_file="$tmp_dir/violations.tsv"

awk -F',' '
  NF >= 12 {
    scenario = $2
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", scenario)
    avg_ms = $9
    if (scenario != "" && avg_ms ~ /^-?[0-9]+([.][0-9]+)?$/) {
      latest_avg[scenario] = avg_ms
    }
  }
  END {
    for (scenario in latest_avg) {
      printf "%s\t%s\n", scenario, latest_avg[scenario]
    }
  }
' "$RESULTS_FILE" | sort -u > "$latest_file"

awk -F '\t' -v budget="$BUDGET_MS" '
  NF >= 2 {
    avg = $2 + 0
    if (avg > (budget + 0)) {
      printf "%s\t%s\t%s\n", $1, $2, budget
    }
  }
' "$latest_file" > "$violations_file"

scenarios_checked="$(wc -l < "$latest_file" | tr -d ' ')"
violations_count="$(wc -l < "$violations_file" | tr -d ' ')"
status="ok"
if [ "$violations_count" -gt 0 ]; then
  status="budget_exceeded"
fi

{
  echo "# Recursive Runtime Budget"
  echo
  echo "- status: $status"
  echo "- results_file: $RESULTS_FILE"
  echo "- profile: $PROFILE"
  echo "- budget_ms: $BUDGET_MS"
  echo "- scenarios_checked: $scenarios_checked"
  echo "- violations: $violations_count"
  echo
  echo "## Violations (scenario | avg_ms | budget_ms)"
  if [ "$violations_count" -eq 0 ]; then
    echo "- none"
  else
    while IFS="$(printf '\t')" read -r scenario avg_ms budget; do
      printf -- "- %s | %s | %s\n" "$scenario" "$avg_ms" "$budget"
    done < "$violations_file"
  fi
} > "$REPORT_FILE"

if [ "$STRICT_MODE" = "true" ] && [ "$status" != "ok" ]; then
  echo "recursive runtime budget exceeded (profile=$PROFILE, budget_ms=$BUDGET_MS): $REPORT_FILE" >&2
  exit 1
fi

echo "wrote recursive runtime budget report: $REPORT_FILE"
