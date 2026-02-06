#!/bin/sh
set -eu

CANDIDATE_FILE=${1:-perf/baselines/java_layered_issue_scenarios.candidate.csv}
RUST_FILE=${2:-perf/results_layered_issue_scenarios.csv}
WINDOW=${3:-3}
THRESHOLD=${4:-0}
REPORT=${5:-perf/java_baseline_candidate_check.md}

JAVA_CANDIDATE_MIN_ROWS=${JAVA_CANDIDATE_MIN_ROWS:-1}
JAVA_CANDIDATE_REQUIRED_SCENARIOS=${JAVA_CANDIDATE_REQUIRED_SCENARIOS:-issue_405,issue_603,issue_680,issue_871,issue_905}
JAVA_CANDIDATE_REQUIRE_PARITY=${JAVA_CANDIDATE_REQUIRE_PARITY:-true}
JAVA_CANDIDATE_STRICT=${JAVA_CANDIDATE_STRICT:-false}
TARGET_BASELINE=${TARGET_BASELINE:-perf/baselines/java_layered_issue_scenarios.csv}

mkdir -p "$(dirname "$REPORT")"

tmp_artifact_report=$(mktemp)
tmp_compare_report=$(mktemp)
tmp_artifact_log=$(mktemp)
tmp_compare_log=$(mktemp)
tmp_parity_log=$(mktemp)
trap 'rm -f "$tmp_artifact_report" "$tmp_compare_report" "$tmp_artifact_log" "$tmp_compare_log" "$tmp_parity_log"' EXIT HUP INT TERM
printf '# candidate artifact check\n' > "$tmp_artifact_report"

status="ready"
reason="candidate passed checks"

if [ ! -s "$CANDIDATE_FILE" ]; then
  status="skipped"
  reason="candidate file is missing or empty"
else
  if ! JAVA_COMPARE_ENABLED=true \
    JAVA_GENERATE_ENABLED=false \
    JAVA_GENERATE_DRY_RUN=false \
    JAVA_ARTIFACT_MIN_ROWS="$JAVA_CANDIDATE_MIN_ROWS" \
    JAVA_ARTIFACT_REQUIRED_SCENARIOS="$JAVA_CANDIDATE_REQUIRED_SCENARIOS" \
    sh scripts/check_java_perf_artifacts.sh "$CANDIDATE_FILE" "$tmp_artifact_report" >"$tmp_artifact_log" 2>&1; then
    status="not_ready"
    reason="artifact validation failed"
  fi

  if [ "$status" = "ready" ]; then
    if ! sh scripts/compare_java_perf_results.sh "$RUST_FILE" "$CANDIDATE_FILE" "$WINDOW" "$tmp_compare_report" >"$tmp_compare_log" 2>&1; then
      status="not_ready"
      reason="compare report generation failed"
    fi
  fi

  if [ "$status" = "ready" ] && [ "$JAVA_CANDIDATE_REQUIRE_PARITY" = "true" ]; then
    if ! sh scripts/check_java_perf_parity.sh "$RUST_FILE" "$CANDIDATE_FILE" "$WINDOW" "$THRESHOLD" >"$tmp_parity_log" 2>&1; then
      status="not_ready"
      reason="parity check failed"
    fi
  fi
fi

{
  echo "# Java Baseline Candidate Check"
  echo
  echo "- status: $status"
  echo "- reason: $reason"
  echo "- candidate: \`$CANDIDATE_FILE\`"
  echo "- rust file: \`$RUST_FILE\`"
  echo "- target baseline: \`$TARGET_BASELINE\`"
  echo "- require parity: \`$JAVA_CANDIDATE_REQUIRE_PARITY\`"
  echo "- threshold: \`$THRESHOLD\`"
  echo
  if [ "$status" = "ready" ]; then
    echo "## Next Action"
    echo
    echo "- promote candidate: \`sh scripts/update_java_perf_baseline.sh \"$CANDIDATE_FILE\" \"$TARGET_BASELINE\"\`"
  else
    echo "## Diagnostic Logs"
    echo
    if [ -s "$tmp_artifact_log" ]; then
      echo "### Artifact Validation"
      echo
      echo '```'
      cat "$tmp_artifact_log"
      echo '```'
      echo
    fi
    if [ -s "$tmp_compare_log" ]; then
      echo "### Compare Generation"
      echo
      echo '```'
      cat "$tmp_compare_log"
      echo '```'
      echo
    fi
    if [ -s "$tmp_parity_log" ]; then
      echo "### Parity Check"
      echo
      echo '```'
      cat "$tmp_parity_log"
      echo '```'
      echo
    fi
  fi
} > "$REPORT"

echo "wrote $REPORT ($status)"

if [ "$status" != "ready" ] && [ "$JAVA_CANDIDATE_STRICT" = "true" ]; then
  exit 1
fi
