#!/bin/sh
set -eu

RESULTS_REPORT=${1:-perf/java_vs_rust.md}
BASELINE_REPORT=${2:-perf/java_vs_rust_baseline.md}
JAVA_RESULTS_FILE=${3:-perf/java_results_layered_issue_scenarios.csv}
JAVA_BASELINE_FILE=${4:-perf/baselines/java_layered_issue_scenarios.csv}
OUTPUT=${5:-perf/java_perf_status.md}
RUST_RESULTS_FILE=${RUST_RESULTS_FILE:-perf/results_layered_issue_scenarios.csv}
JAVA_BASELINE_CANDIDATE_FILE=${JAVA_BASELINE_CANDIDATE_FILE:-perf/baselines/java_layered_issue_scenarios.candidate.csv}
JAVA_BASELINE_CANDIDATE_REPORT=${JAVA_BASELINE_CANDIDATE_REPORT:-perf/java_baseline_candidate_status.md}
JAVA_BASELINE_CANDIDATE_CHECK_REPORT=${JAVA_BASELINE_CANDIDATE_CHECK_REPORT:-perf/java_baseline_candidate_check.md}

exists_or_no() {
  file=$1
  if [ -s "$file" ]; then
    echo "yes"
  else
    echo "no"
  fi
}

results_report_exists=$(exists_or_no "$RESULTS_REPORT")
baseline_report_exists=$(exists_or_no "$BASELINE_REPORT")
java_results_exists=$(exists_or_no "$JAVA_RESULTS_FILE")
java_baseline_exists=$(exists_or_no "$JAVA_BASELINE_FILE")
java_candidate_exists=$(exists_or_no "$JAVA_BASELINE_CANDIDATE_FILE")
java_candidate_report_exists=$(exists_or_no "$JAVA_BASELINE_CANDIDATE_REPORT")
java_candidate_check_report_exists=$(exists_or_no "$JAVA_BASELINE_CANDIDATE_CHECK_REPORT")
candidate_matches_baseline="no"
if [ "$java_candidate_exists" = "yes" ] && [ "$java_baseline_exists" = "yes" ]; then
  if cmp -s "$JAVA_BASELINE_CANDIDATE_FILE" "$JAVA_BASELINE_FILE"; then
    candidate_matches_baseline="yes"
  fi
fi
candidate_check_status="unknown"
if [ "$java_candidate_check_report_exists" = "yes" ]; then
  if rg -n "^- status: ready$" "$JAVA_BASELINE_CANDIDATE_CHECK_REPORT" >/dev/null 2>&1; then
    candidate_check_status="ready"
  elif rg -n "^- status: not_ready$" "$JAVA_BASELINE_CANDIDATE_CHECK_REPORT" >/dev/null 2>&1; then
    candidate_check_status="not_ready"
  elif rg -n "^- status: skipped$" "$JAVA_BASELINE_CANDIDATE_CHECK_REPORT" >/dev/null 2>&1; then
    candidate_check_status="skipped"
  fi
fi

results_skip_reason="none"
if [ "$results_report_exists" = "yes" ]; then
  if rg -n "dry-run mode is enabled|generation failed and allowed to continue|skipped because no Java CSV" "$RESULTS_REPORT" >/dev/null 2>&1; then
    results_skip_reason="results compare skipped"
  fi
fi

mkdir -p "$(dirname "$OUTPUT")"
{
  echo "# Java Perf Status"
  echo
  echo "- results report: \`$RESULTS_REPORT\` ($results_report_exists)"
  echo "- baseline report: \`$BASELINE_REPORT\` ($baseline_report_exists)"
  echo "- java results csv: \`$JAVA_RESULTS_FILE\` ($java_results_exists)"
  echo "- java baseline csv: \`$JAVA_BASELINE_FILE\` ($java_baseline_exists)"
  echo "- rust layered issue csv: \`$RUST_RESULTS_FILE\`"
  echo "- java baseline candidate csv: \`$JAVA_BASELINE_CANDIDATE_FILE\` ($java_candidate_exists)"
  echo "- java baseline candidate report: \`$JAVA_BASELINE_CANDIDATE_REPORT\` ($java_candidate_report_exists)"
  echo "- java baseline candidate check report: \`$JAVA_BASELINE_CANDIDATE_CHECK_REPORT\` ($java_candidate_check_report_exists)"
  echo "- java baseline candidate check status: $candidate_check_status"
  echo "- java baseline candidate equals baseline: $candidate_matches_baseline"
  echo "- results skip reason: $results_skip_reason"
  echo
  if [ "$java_candidate_exists" = "yes" ] && [ "$candidate_check_status" = "ready" ]; then
    echo "## Next Action"
    echo
    if [ "$candidate_matches_baseline" = "yes" ]; then
      echo "- baseline is already synchronized with the ready candidate."
      echo "- re-run compare in \`java_compare_mode=both\` with desired parity gates when Rust perf inputs change."
    else
      echo "- promote candidate with \`sh scripts/update_java_perf_baseline.sh \"$JAVA_BASELINE_CANDIDATE_FILE\" \"$JAVA_BASELINE_FILE\"\`"
      echo "- then re-run compare in \`java_compare_mode=both\` with desired parity gates."
    fi
  elif [ "$java_candidate_exists" = "yes" ] && [ "$candidate_check_status" = "unknown" ]; then
    echo "## Next Action"
    echo
    echo "- candidate exists but readiness has not been checked; run \`sh scripts/check_java_baseline_candidate.sh \"$JAVA_BASELINE_CANDIDATE_FILE\" \"$RUST_RESULTS_FILE\" 3 0 \"$JAVA_BASELINE_CANDIDATE_CHECK_REPORT\"\`."
    echo "- after readiness is \`ready\`, promote candidate and re-run compare in \`java_compare_mode=both\`."
  elif [ "$java_candidate_exists" = "yes" ]; then
    echo "## Next Action"
    echo
    echo "- candidate exists but is not ready for promotion; inspect \`$JAVA_BASELINE_CANDIDATE_CHECK_REPORT\`."
    echo "- refresh Java results and re-run candidate export/check steps."
  elif [ "$java_results_exists" = "yes" ]; then
    echo "## Next Action"
    echo
    echo "- no candidate file found; create one with \`sh scripts/export_java_baseline_candidate.sh \"$JAVA_RESULTS_FILE\" \"$JAVA_BASELINE_CANDIDATE_FILE\" \"$JAVA_BASELINE_CANDIDATE_REPORT\"\`"
    echo "- after export, promote baseline and re-run compare in \`java_compare_mode=both\`."
  else
    echo "## Next Action"
    echo
    echo "- Java results CSV not available. Re-run generation with network access."
    echo "- suggested toggles: increase \`java_generate_retries\`, keep \`java_allow_generate_failure=false\` for strict gate."
  fi
} > "$OUTPUT"

echo "wrote $OUTPUT"
