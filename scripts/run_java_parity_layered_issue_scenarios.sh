#!/bin/sh
# ============================================================================
# run_java_parity_layered_issue_scenarios.sh — Java layered issue parity bench
#
# Runs the external ELK Java layered benchmark test (LayeredIssueParityBenchTest)
# via Tycho.  Benchmark test source is temporarily injected and cleaned up.
#
# Usage:
#   sh scripts/run_java_parity_layered_issue_scenarios.sh [scenarios] [iterations] [warmup] [output]
#
# Environment variables: all JAVA_PARITY_* vars are supported.
# ============================================================================
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

EJC_PREFIX=JAVA_PARITY
# shellcheck source=java/elk_java_common.sh
. "$SCRIPT_DIR/java/elk_java_common.sh"

# ========================== Configuration ===================================

SCENARIOS=${1:-issue_405,issue_603,issue_680,issue_871,issue_905}
ITERATIONS=${2:-20}
WARMUP=${3:-3}
OUTPUT=${4:-tests/java_results_layered_issue_scenarios.csv}

JAVA_PARITY_MVN_BIN=${JAVA_PARITY_MVN_BIN:-mvn}
JAVA_PARITY_BUILD_PLUGINS=${JAVA_PARITY_BUILD_PLUGINS:-true}
JAVA_PARITY_EXTERNAL_ELK_ROOT=${JAVA_PARITY_EXTERNAL_ELK_ROOT:-$REPO_ROOT/external/elk}
JAVA_PARITY_EXTERNAL_ISOLATE=${JAVA_PARITY_EXTERNAL_ISOLATE:-true}
JAVA_PARITY_EXTERNAL_WORKTREE_ROOT=${JAVA_PARITY_EXTERNAL_WORKTREE_ROOT:-${TMPDIR:-/tmp}}
JAVA_PARITY_PREPARE_POM=${JAVA_PARITY_PREPARE_POM:-}
JAVA_PARITY_PREPARE_MODULES=${JAVA_PARITY_PREPARE_MODULES:-}
JAVA_PARITY_TEST_POM=${JAVA_PARITY_TEST_POM:-}
JAVA_PARITY_TEST_MODULES=${JAVA_PARITY_TEST_MODULES:-../test/org.eclipse.elk.alg.test,../test/org.eclipse.elk.alg.layered.test}
JAVA_PARITY_TEST_CLASS=${JAVA_PARITY_TEST_CLASS:-LayeredIssueParityBenchTest}
JAVA_PARITY_TEST_METHOD=${JAVA_PARITY_TEST_METHOD:-}
JAVA_PARITY_TEST_GOAL=${JAVA_PARITY_TEST_GOAL:-verify}
JAVA_PARITY_BENCH_INJECT=${JAVA_PARITY_BENCH_INJECT:-true}
JAVA_PARITY_BENCH_SOURCE=${JAVA_PARITY_BENCH_SOURCE:-$SCRIPT_DIR/java/LayeredIssueParityBenchTest.java}
JAVA_PARITY_BENCH_DEST=${JAVA_PARITY_BENCH_DEST:-}
JAVA_PARITY_BENCH_CLEANUP=${JAVA_PARITY_BENCH_CLEANUP:-true}
JAVA_PARITY_PREPARE_ARGS=${JAVA_PARITY_PREPARE_ARGS:--DskipTests -DskipITs}
JAVA_PARITY_DRY_RUN=${JAVA_PARITY_DRY_RUN:-false}
JAVA_PARITY_MVN_ARGS=${JAVA_PARITY_MVN_ARGS:-}
JAVA_PARITY_MVN_LOCAL_REPO=${JAVA_PARITY_MVN_LOCAL_REPO:-${TMPDIR:-/tmp}/elk-java-parity-m2}
JAVA_PARITY_RETRIES=${JAVA_PARITY_RETRIES:-0}
JAVA_PARITY_RETRY_DELAY_SECS=${JAVA_PARITY_RETRY_DELAY_SECS:-3}
JAVA_PARITY_SKIP_DNS_CHECK=${JAVA_PARITY_SKIP_DNS_CHECK:-false}
JAVA_PARITY_REQUIRED_HOSTS=${JAVA_PARITY_REQUIRED_HOSTS:-repo.eclipse.org,repo.maven.apache.org}

# Resolve output to absolute path
ejc_resolve_to_absolute "$OUTPUT" "$(pwd)"; OUTPUT=$_ejc_val

# ========================== Script-specific state ===========================

EJC_ELK_ROOT=$JAVA_PARITY_EXTERNAL_ELK_ROOT

# ========================== Cleanup =========================================

_layered_cleanup() {
  ejc_restore_java_file
}

ejc_register_cleanup _layered_cleanup

# ========================== Validation ======================================

ejc_validate_integer JAVA_PARITY_RETRIES "$JAVA_PARITY_RETRIES"
ejc_validate_integer JAVA_PARITY_RETRY_DELAY_SECS "$JAVA_PARITY_RETRY_DELAY_SECS"
ejc_validate_maven

# ========================== Isolation =======================================

ejc_create_isolation parity

# ========================== Resolve ELK-relative paths ======================

if [ -z "$JAVA_PARITY_PREPARE_POM" ]; then
  JAVA_PARITY_PREPARE_POM="$EJC_ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_TEST_POM" ]; then
  JAVA_PARITY_TEST_POM="$EJC_ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_BENCH_DEST" ]; then
  JAVA_PARITY_BENCH_DEST="$EJC_ELK_ROOT/test/org.eclipse.elk.alg.layered.test/src/org/eclipse/elk/alg/layered/issues/LayeredIssueParityBenchTest.java"
fi

# ========================== Inject bench test class =========================

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_BENCH_INJECT" = "true" ]; then
  case "$JAVA_PARITY_TEST_CLASS" in
    *LayeredIssueParityBenchTest)
      if [ ! -f "$JAVA_PARITY_BENCH_DEST" ]; then
        ejc_inject_java_file "$JAVA_PARITY_BENCH_SOURCE" "$JAVA_PARITY_BENCH_DEST"
      fi
      ;;
  esac
fi

# ========================== DNS preflight ===================================

ejc_dns_preflight

mkdir -p "$(dirname "$OUTPUT")"

# ========================== Build plugins ===================================

ejc_mvn_build_plugins "$JAVA_PARITY_PREPARE_POM"

# ========================== Run Tycho test ==================================

TEST_SELECTOR="$JAVA_PARITY_TEST_CLASS"
if [ -n "$JAVA_PARITY_TEST_METHOD" ]; then
  TEST_SELECTOR="$TEST_SELECTOR#$JAVA_PARITY_TEST_METHOD"
fi
TYCHO_TEST_ARG_LINE="-Delk.parity.run=true -Delk.parity.scenarios=$SCENARIOS -Delk.parity.iterations=$ITERATIONS -Delk.parity.warmup=$WARMUP -Delk.parity.output=$OUTPUT"

ejc_resolve_var MVN_BIN mvn; _mvn=$_ejc_val
set -- \
  "$_mvn" \
  -f "$JAVA_PARITY_TEST_POM" \
  -pl "$JAVA_PARITY_TEST_MODULES" \
  -am \
  "-Dtest=$TEST_SELECTOR" \
  -DfailIfNoTests=false \
  "-Dtycho.testArgLine=$TYCHO_TEST_ARG_LINE"

if [ -n "$JAVA_PARITY_MVN_LOCAL_REPO" ]; then
  set -- "$@" "-Dmaven.repo.local=$JAVA_PARITY_MVN_LOCAL_REPO"
fi
if [ -n "$JAVA_PARITY_MVN_ARGS" ]; then
  # shellcheck disable=SC2086
  set -- "$@" $JAVA_PARITY_MVN_ARGS
fi
set -- "$@" "$JAVA_PARITY_TEST_GOAL"
ejc_run_cmd "$@"
