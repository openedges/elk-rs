#!/bin/sh
set -eu

JAVA_FILE=${1:-parity/java_results_layered_issue_scenarios.csv}
REPORT_FILE=${2:-parity/java_vs_rust.md}

JAVA_COMPARE_ENABLED=${JAVA_COMPARE_ENABLED:-true}
JAVA_GENERATE_ENABLED=${JAVA_GENERATE_ENABLED:-false}
JAVA_GENERATE_DRY_RUN=${JAVA_GENERATE_DRY_RUN:-false}
JAVA_ARTIFACT_MIN_ROWS=${JAVA_ARTIFACT_MIN_ROWS:-1}
JAVA_ARTIFACT_REQUIRED_SCENARIOS=${JAVA_ARTIFACT_REQUIRED_SCENARIOS:-}
JAVA_ARTIFACT_SCENARIO_COL=${JAVA_ARTIFACT_SCENARIO_COL:-2}

case "$JAVA_ARTIFACT_MIN_ROWS" in
  ''|*[!0-9]*)
    echo "invalid JAVA_ARTIFACT_MIN_ROWS (must be non-negative integer): $JAVA_ARTIFACT_MIN_ROWS" >&2
    exit 1
    ;;
esac

required_scenario_count=$(printf '%s\n' "$JAVA_ARTIFACT_REQUIRED_SCENARIOS" | awk -F',' '
  {
    for (i = 1; i <= NF; i++) {
      value = $i
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      if (value != "") {
        count += 1
      }
    }
  }
  END { print count + 0 }
')
effective_min_rows=$JAVA_ARTIFACT_MIN_ROWS
if [ "$required_scenario_count" -gt "$effective_min_rows" ]; then
  effective_min_rows=$required_scenario_count
fi

if [ "$JAVA_COMPARE_ENABLED" != "true" ]; then
  echo "skip java artifact check: JAVA_COMPARE_ENABLED=$JAVA_COMPARE_ENABLED"
  exit 0
fi

if [ ! -s "$REPORT_FILE" ]; then
  echo "missing or empty Java compare report: $REPORT_FILE" >&2
  exit 1
fi

if [ "$JAVA_GENERATE_ENABLED" = "true" ] && [ "$JAVA_GENERATE_DRY_RUN" != "true" ]; then
  if [ ! -s "$JAVA_FILE" ]; then
    echo "missing or empty generated Java parity CSV: $JAVA_FILE" >&2
    exit 1
  fi
fi

if [ -s "$JAVA_FILE" ]; then
  row_count=$(awk -F',' -v sc="$JAVA_ARTIFACT_SCENARIO_COL" '
    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      return value
    }
    {
      scenario = trim($sc)
      if (scenario == "" || tolower(scenario) == "scenario") {
        next
      }
      count += 1
    }
    END { print count + 0 }
  ' "$JAVA_FILE")
  if [ "$row_count" -lt "$effective_min_rows" ]; then
    echo "java parity CSV data row count too small: rows=$row_count min=$effective_min_rows configured_min=$JAVA_ARTIFACT_MIN_ROWS required_scenarios=$required_scenario_count file=$JAVA_FILE" >&2
    exit 1
  fi

  if [ -n "$JAVA_ARTIFACT_REQUIRED_SCENARIOS" ]; then
    tmp_scenarios=$(mktemp)
    trap 'rm -f "$tmp_scenarios"' EXIT HUP INT TERM
    awk -F',' -v sc="$JAVA_ARTIFACT_SCENARIO_COL" '
      function trim(value) {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
        return value
      }
      {
        scenario = trim($sc)
        if (scenario != "" && tolower(scenario) != "scenario") {
          print scenario
        }
      }
    ' "$JAVA_FILE" | sort -u > "$tmp_scenarios"

    OLD_IFS=${IFS}
    IFS=','
    # shellcheck disable=SC2086
    set -- $JAVA_ARTIFACT_REQUIRED_SCENARIOS
    IFS=${OLD_IFS}
    for required in "$@"; do
      required_trimmed=$(printf '%s' "$required" | awk '{ gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print }')
      if [ -z "$required_trimmed" ]; then
        continue
      fi
      if ! grep -Fxq "$required_trimmed" "$tmp_scenarios"; then
        echo "java parity CSV missing required scenario '$required_trimmed': $JAVA_FILE" >&2
        exit 1
      fi
    done
  fi
fi

echo "java parity artifacts verified: report=$REPORT_FILE java_csv=$JAVA_FILE"
