#!/bin/sh
set -eu

JAVA_FILE=${1:-tests/java_results_layered_issue_scenarios.csv}
WINDOW=${2:-3}
THRESHOLD=${3:-0}
OUTPUT=${4:-tests/java_vs_rust_layered_issue.md}

LAYERED_ISSUE_SCENARIOS=${LAYERED_ISSUE_SCENARIOS:-issue_405,issue_603,issue_680,issue_871,issue_905}
LAYERED_ISSUE_ITERATIONS=${LAYERED_ISSUE_ITERATIONS:-20}
LAYERED_ISSUE_WARMUP=${LAYERED_ISSUE_WARMUP:-3}
LAYERED_ISSUE_OUTPUT=${LAYERED_ISSUE_OUTPUT:-tests/results_layered_issue_scenarios.csv}
LAYERED_ISSUE_SKIP_RUST_RUN=${LAYERED_ISSUE_SKIP_RUST_RUN:-false}
JAVA_PARITY_GENERATE=${JAVA_PARITY_GENERATE:-false}
JAVA_PARITY_DRY_RUN=${JAVA_PARITY_DRY_RUN:-false}
JAVA_PARITY_SCENARIOS=${JAVA_PARITY_SCENARIOS:-$LAYERED_ISSUE_SCENARIOS}
JAVA_PARITY_ITERATIONS=${JAVA_PARITY_ITERATIONS:-$LAYERED_ISSUE_ITERATIONS}
JAVA_PARITY_WARMUP=${JAVA_PARITY_WARMUP:-$LAYERED_ISSUE_WARMUP}
JAVA_PARITY_OUTPUT=${JAVA_PARITY_OUTPUT:-$JAVA_FILE}
JAVA_PARITY_RESET_OUTPUT=${JAVA_PARITY_RESET_OUTPUT:-true}
JAVA_PARITY_VERIFY_ARTIFACTS=${JAVA_PARITY_VERIFY_ARTIFACTS:-true}
JAVA_PARITY_ALLOW_GENERATE_FAILURE=${JAVA_PARITY_ALLOW_GENERATE_FAILURE:-false}
JAVA_ARTIFACT_MIN_ROWS=${JAVA_ARTIFACT_MIN_ROWS:-1}
JAVA_ARTIFACT_REQUIRED_SCENARIOS=${JAVA_ARTIFACT_REQUIRED_SCENARIOS:-$LAYERED_ISSUE_SCENARIOS}
JAVA_PARITY_MVN_LOCAL_REPO=${JAVA_PARITY_MVN_LOCAL_REPO:-}
JAVA_PARITY_COMPARE_MODE=${JAVA_PARITY_COMPARE_MODE:-results}
JAVA_PARITY_SCENARIO_THRESHOLDS_FILE=${JAVA_PARITY_SCENARIO_THRESHOLDS_FILE:-tests/java_parity_thresholds.csv}
JAVA_BASELINE_FILE=${JAVA_BASELINE_FILE:-tests/baselines/java_layered_issue_scenarios.csv}
JAVA_BASELINE_OUTPUT=${JAVA_BASELINE_OUTPUT:-tests/java_vs_rust_baseline.md}
JAVA_BASELINE_THRESHOLD=${JAVA_BASELINE_THRESHOLD:-$THRESHOLD}
JAVA_RESULTS_PARITY_GATE=${JAVA_RESULTS_PARITY_GATE:-true}
JAVA_BASELINE_PARITY_GATE=${JAVA_BASELINE_PARITY_GATE:-true}

run_results_compare=false
run_baseline_compare=false
case "$JAVA_PARITY_COMPARE_MODE" in
  results)
    run_results_compare=true
    ;;
  baseline)
    run_baseline_compare=true
    ;;
  both)
    run_results_compare=true
    run_baseline_compare=true
    ;;
  *)
    echo "unsupported JAVA_PARITY_COMPARE_MODE: $JAVA_PARITY_COMPARE_MODE (expected: results|baseline|both)" >&2
    exit 1
    ;;
esac

run_java_parity_gate() {
  rust_file=$1
  java_file=$2
  threshold=$3

  if [ -n "$JAVA_PARITY_SCENARIO_THRESHOLDS_FILE" ] && [ -f "$JAVA_PARITY_SCENARIO_THRESHOLDS_FILE" ]; then
    sh scripts/check_java_parity_scenarios.sh \
      "$rust_file" \
      "$java_file" \
      "$WINDOW" \
      "$JAVA_PARITY_SCENARIO_THRESHOLDS_FILE"
  else
    if [ -n "$JAVA_PARITY_SCENARIO_THRESHOLDS_FILE" ]; then
      echo "warning: missing scenario thresholds file ($JAVA_PARITY_SCENARIO_THRESHOLDS_FILE), fallback to global threshold gate" >&2
    fi
    sh scripts/check_java_parity.sh \
      "$rust_file" \
      "$java_file" \
      "$WINDOW" \
      "$threshold"
  fi
}

if [ "$JAVA_PARITY_GENERATE" = "true" ] && [ "$run_results_compare" != "true" ]; then
  echo "skip java generation because JAVA_PARITY_COMPARE_MODE=$JAVA_PARITY_COMPARE_MODE does not use results compare" >&2
  JAVA_PARITY_GENERATE=false
fi

if [ "$JAVA_PARITY_GENERATE" = "true" ] && [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ -z "$JAVA_PARITY_MVN_LOCAL_REPO" ]; then
  JAVA_PARITY_MVN_LOCAL_REPO="${TMPDIR:-/tmp}/m2-java-parity-${USER:-user}-$$"
fi

if [ "$LAYERED_ISSUE_SKIP_RUST_RUN" = "true" ]; then
  if [ ! -s "$LAYERED_ISSUE_OUTPUT" ]; then
    echo "missing rust layered issue output while LAYERED_ISSUE_SKIP_RUST_RUN=true: $LAYERED_ISSUE_OUTPUT" >&2
    exit 1
  fi
else
  sh scripts/run_parity_layered_issue_scenarios.sh \
    "$LAYERED_ISSUE_SCENARIOS" \
    "$LAYERED_ISSUE_ITERATIONS" \
    "$LAYERED_ISSUE_WARMUP" \
    "$LAYERED_ISSUE_OUTPUT"
fi

if [ "$JAVA_PARITY_GENERATE" = "true" ]; then
  if [ "$JAVA_PARITY_RESET_OUTPUT" = "true" ]; then
    rm -f "$JAVA_PARITY_OUTPUT"
  fi
  if JAVA_PARITY_MVN_LOCAL_REPO="$JAVA_PARITY_MVN_LOCAL_REPO" \
    sh scripts/run_java_parity_layered_issue_scenarios.sh \
      "$JAVA_PARITY_SCENARIOS" \
      "$JAVA_PARITY_ITERATIONS" \
      "$JAVA_PARITY_WARMUP" \
      "$JAVA_PARITY_OUTPUT"; then
    JAVA_FILE=$JAVA_PARITY_OUTPUT
    java_generate_failed=false
  else
    java_generate_failed=true
    if [ "$JAVA_PARITY_ALLOW_GENERATE_FAILURE" != "true" ]; then
      exit 1
    fi
    echo "java parity generation failed but continuing because JAVA_PARITY_ALLOW_GENERATE_FAILURE=true" >&2
  fi
else
  java_generate_failed=false
fi

if [ "$run_results_compare" = "true" ]; then
  results_artifact_check_required=true
  if [ "$JAVA_PARITY_GENERATE" = "true" ] && [ "$JAVA_PARITY_DRY_RUN" = "true" ] && [ ! -s "$JAVA_FILE" ]; then
    mkdir -p "$(dirname "$OUTPUT")"
    {
      echo "# Java vs Rust Layered Issue Perf"
      echo
      echo "- Java generation dry-run mode is enabled."
      echo "- compare/parity checks are skipped because no Java CSV was produced."
      echo "- expected java csv path: \`$JAVA_FILE\`"
    } > "$OUTPUT"
  elif [ "$java_generate_failed" = "true" ] && [ ! -s "$JAVA_FILE" ]; then
    results_artifact_check_required=false
    mkdir -p "$(dirname "$OUTPUT")"
    {
      echo "# Java vs Rust Layered Issue Perf"
      echo
      echo "- Java generation failed and allowed to continue."
      echo "- compare/parity checks are skipped because no Java CSV was produced."
      echo "- expected java csv path: \`$JAVA_FILE\`"
    } > "$OUTPUT"
  else
    sh scripts/compare_java_parity_results.sh \
      "$LAYERED_ISSUE_OUTPUT" \
      "$JAVA_FILE" \
      "$WINDOW" \
      "$OUTPUT"

    if [ "$JAVA_RESULTS_PARITY_GATE" = "true" ]; then
      run_java_parity_gate "$LAYERED_ISSUE_OUTPUT" "$JAVA_FILE" "$THRESHOLD"
    fi
  fi
else
  results_artifact_check_required=false
fi

if [ "$run_baseline_compare" = "true" ]; then
  sh scripts/compare_java_parity_results.sh \
    "$LAYERED_ISSUE_OUTPUT" \
    "$JAVA_BASELINE_FILE" \
    "$WINDOW" \
    "$JAVA_BASELINE_OUTPUT"

  if [ "$JAVA_BASELINE_PARITY_GATE" = "true" ]; then
    run_java_parity_gate "$LAYERED_ISSUE_OUTPUT" "$JAVA_BASELINE_FILE" "$JAVA_BASELINE_THRESHOLD"
  fi
fi

if [ "$JAVA_PARITY_VERIFY_ARTIFACTS" = "true" ] && [ "$run_results_compare" = "true" ] && [ "$results_artifact_check_required" = "true" ]; then
  JAVA_COMPARE_ENABLED=true \
  JAVA_GENERATE_ENABLED="$JAVA_PARITY_GENERATE" \
  JAVA_GENERATE_DRY_RUN="$JAVA_PARITY_DRY_RUN" \
  JAVA_ARTIFACT_MIN_ROWS="$JAVA_ARTIFACT_MIN_ROWS" \
  JAVA_ARTIFACT_REQUIRED_SCENARIOS="$JAVA_ARTIFACT_REQUIRED_SCENARIOS" \
  sh scripts/check_java_parity_artifacts.sh "$JAVA_FILE" "$OUTPUT"
fi

if [ "$JAVA_PARITY_VERIFY_ARTIFACTS" = "true" ] && [ "$run_baseline_compare" = "true" ]; then
  JAVA_COMPARE_ENABLED=true \
  JAVA_GENERATE_ENABLED=false \
  JAVA_GENERATE_DRY_RUN=false \
  JAVA_ARTIFACT_MIN_ROWS="$JAVA_ARTIFACT_MIN_ROWS" \
  JAVA_ARTIFACT_REQUIRED_SCENARIOS="$JAVA_ARTIFACT_REQUIRED_SCENARIOS" \
  sh scripts/check_java_parity_artifacts.sh "$JAVA_BASELINE_FILE" "$JAVA_BASELINE_OUTPUT"
fi
