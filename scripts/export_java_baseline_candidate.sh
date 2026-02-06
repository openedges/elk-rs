#!/bin/sh
set -eu

SOURCE=${1:-perf/java_results_layered_issue_scenarios.csv}
TARGET=${2:-perf/baselines/java_layered_issue_scenarios.candidate.csv}
REPORT=${3:-perf/java_baseline_candidate_status.md}

JAVA_CANDIDATE_MIN_ROWS=${JAVA_CANDIDATE_MIN_ROWS:-1}
JAVA_CANDIDATE_REQUIRED_SCENARIOS=${JAVA_CANDIDATE_REQUIRED_SCENARIOS:-}
JAVA_CANDIDATE_STRICT=${JAVA_CANDIDATE_STRICT:-false}

mkdir -p "$(dirname "$REPORT")"

if [ ! -s "$SOURCE" ]; then
  {
    echo "# Java Baseline Candidate"
    echo
    echo "- status: skipped"
    echo "- reason: missing or empty source CSV"
    echo "- source: \`$SOURCE\`"
    echo "- target: \`$TARGET\`"
  } > "$REPORT"
  echo "wrote $REPORT (skipped)"
  exit 0
fi

tmp_report=$(mktemp)
trap 'rm -f "$tmp_report"' EXIT HUP INT TERM
printf '# candidate validation\n' > "$tmp_report"

validation_output=""
if ! validation_output=$(
  JAVA_COMPARE_ENABLED=true \
  JAVA_GENERATE_ENABLED=true \
  JAVA_GENERATE_DRY_RUN=false \
  JAVA_ARTIFACT_MIN_ROWS="$JAVA_CANDIDATE_MIN_ROWS" \
  JAVA_ARTIFACT_REQUIRED_SCENARIOS="$JAVA_CANDIDATE_REQUIRED_SCENARIOS" \
  sh scripts/check_java_perf_artifacts.sh "$SOURCE" "$tmp_report" 2>&1
); then
  {
    echo "# Java Baseline Candidate"
    echo
    echo "- status: invalid"
    echo "- reason: candidate failed artifact policy check"
    echo "- source: \`$SOURCE\`"
    echo "- target: \`$TARGET\`"
    echo "- min_rows: \`$JAVA_CANDIDATE_MIN_ROWS\`"
    echo "- required_scenarios: \`$JAVA_CANDIDATE_REQUIRED_SCENARIOS\`"
    echo
    echo "## Validation Output"
    echo
    echo '```'
    printf '%s\n' "$validation_output"
    echo '```'
  } > "$REPORT"
  echo "wrote $REPORT (invalid candidate)"
  if [ "$JAVA_CANDIDATE_STRICT" = "true" ]; then
    exit 1
  fi
  exit 0
fi

mkdir -p "$(dirname "$TARGET")"
cp "$SOURCE" "$TARGET"

{
  echo "# Java Baseline Candidate"
  echo
  echo "- status: updated"
  echo "- source: \`$SOURCE\`"
  echo "- target: \`$TARGET\`"
  echo "- min_rows: \`$JAVA_CANDIDATE_MIN_ROWS\`"
  echo "- required_scenarios: \`$JAVA_CANDIDATE_REQUIRED_SCENARIOS\`"
} > "$REPORT"

echo "wrote $REPORT (updated baseline candidate)"
