#!/bin/sh
# ============================================================================
# run_java_model_benchmark.sh — Java model benchmark runner (5-way comparison)
#
# Follows the same isolation/injection pattern as run_java_parity_layered_issue_scenarios.sh
# but targets ElkModelBenchTest (JSON-based benchmark via RecursiveGraphLayoutEngine).
#
# Usage:
#   sh scripts/run_java_model_benchmark.sh [mode] [iterations] [warmup] [output]
#
# Environment variables:
#   JAVA_BENCH_MODE, JAVA_BENCH_MANIFEST, JAVA_BENCH_ITERATIONS,
#   JAVA_BENCH_WARMUP, JAVA_BENCH_OUTPUT, JAVA_BENCH_LIMIT
#   All JAVA_PARITY_* env vars are also supported for Maven/isolation.
# ============================================================================
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

EJC_PREFIX=JAVA_PARITY
# shellcheck source=java/elk_java_common.sh
. "$SCRIPT_DIR/java/elk_java_common.sh"

# ========================== Configuration ===================================

MODE=${1:-${JAVA_BENCH_MODE:-models}}
ITERATIONS=${2:-${JAVA_BENCH_ITERATIONS:-20}}
WARMUP=${3:-${JAVA_BENCH_WARMUP:-3}}
OUTPUT=${4:-${JAVA_BENCH_OUTPUT:-tests/perf/java.csv}}

JAVA_BENCH_MANIFEST=${JAVA_BENCH_MANIFEST:-$REPO_ROOT/parity/model_parity/java/java_manifest.tsv}
JAVA_BENCH_LIMIT=${JAVA_BENCH_LIMIT:-50}

JAVA_PARITY_MVN_BIN=${JAVA_PARITY_MVN_BIN:-mvn}
JAVA_PARITY_BUILD_PLUGINS=${JAVA_PARITY_BUILD_PLUGINS:-true}
JAVA_PARITY_EXTERNAL_ELK_ROOT=${JAVA_PARITY_EXTERNAL_ELK_ROOT:-$REPO_ROOT/external/elk}
JAVA_PARITY_EXTERNAL_ISOLATE=${JAVA_PARITY_EXTERNAL_ISOLATE:-true}
JAVA_PARITY_EXTERNAL_WORKTREE_ROOT=${JAVA_PARITY_EXTERNAL_WORKTREE_ROOT:-${TMPDIR:-/tmp}}
JAVA_PARITY_PREPARE_ARGS=${JAVA_PARITY_PREPARE_ARGS:--DskipTests -DskipITs}
JAVA_PARITY_DRY_RUN=${JAVA_PARITY_DRY_RUN:-false}
JAVA_PARITY_MVN_ARGS=${JAVA_PARITY_MVN_ARGS:-}
JAVA_PARITY_MVN_LOCAL_REPO=${JAVA_PARITY_MVN_LOCAL_REPO:-${TMPDIR:-/tmp}/elk-java-parity-m2}
JAVA_PARITY_RETRIES=${JAVA_PARITY_RETRIES:-0}
JAVA_PARITY_RETRY_DELAY_SECS=${JAVA_PARITY_RETRY_DELAY_SECS:-3}
JAVA_PARITY_SKIP_DNS_CHECK=${JAVA_PARITY_SKIP_DNS_CHECK:-false}
JAVA_PARITY_REQUIRED_HOSTS=${JAVA_PARITY_REQUIRED_HOSTS:-repo.eclipse.org,repo.maven.apache.org}

# Resolve output and manifest to absolute paths
ejc_resolve_to_absolute "$OUTPUT" "$(pwd)"; OUTPUT=$_ejc_val
ejc_resolve_to_absolute "$JAVA_BENCH_MANIFEST" "$(pwd)"; JAVA_BENCH_MANIFEST=$_ejc_val

# ========================== Script-specific state ===========================

EJC_ELK_ROOT=$JAVA_PARITY_EXTERNAL_ELK_ROOT
BENCH_SOURCE="$SCRIPT_DIR/java/ElkModelBenchTest.java"

# ========================== Cleanup =========================================

_bench_cleanup() {
  ejc_restore_java_file
}

ejc_register_cleanup _bench_cleanup

# ========================== Validation ======================================

ejc_validate_integer JAVA_PARITY_RETRIES "$JAVA_PARITY_RETRIES"
ejc_validate_maven

# ========================== Isolation =======================================

ejc_create_isolation bench

# ========================== Resolve ELK-relative paths ======================

PREPARE_POM="$EJC_ELK_ROOT/build/pom.xml"
TEST_POM="$EJC_ELK_ROOT/build/pom.xml"
TEST_MODULES="../test/org.eclipse.elk.graph.json.test"
BENCH_DEST="$EJC_ELK_ROOT/test/org.eclipse.elk.graph.json.test/src/org/eclipse/elk/graph/json/test/ElkModelBenchTest.java"

# ========================== Inject bench test class =========================

if [ "$JAVA_PARITY_DRY_RUN" != "true" ]; then
  if [ ! -f "$BENCH_DEST" ]; then
    ejc_inject_java_file "$BENCH_SOURCE" "$BENCH_DEST"
  fi
fi

# ========================== DNS preflight ===================================

ejc_dns_preflight

mkdir -p "$(dirname "$OUTPUT")"

# ========================== Build plugins ===================================

ejc_mvn_build_plugins "$PREPARE_POM"

# ========================== Run benchmark test ==============================

TYCHO_TEST_ARG_LINE="-Delk.parity.run=true -Delk.bench.mode=$MODE -Delk.bench.iterations=$ITERATIONS -Delk.bench.warmup=$WARMUP -Delk.bench.output=$OUTPUT -Delk.bench.manifest=$JAVA_BENCH_MANIFEST -Delk.bench.limit=$JAVA_BENCH_LIMIT"

ejc_resolve_var MVN_BIN mvn; _mvn=$_ejc_val
set -- \
  "$_mvn" \
  -f "$TEST_POM" \
  -pl "$TEST_MODULES" \
  -am \
  "-Dtest=ElkModelBenchTest" \
  -DfailIfNoTests=false \
  "-Dtycho.testArgLine=$TYCHO_TEST_ARG_LINE"

if [ -n "$JAVA_PARITY_MVN_LOCAL_REPO" ]; then
  set -- "$@" "-Dmaven.repo.local=$JAVA_PARITY_MVN_LOCAL_REPO"
fi
if [ -n "$JAVA_PARITY_MVN_ARGS" ]; then
  # shellcheck disable=SC2086
  set -- "$@" $JAVA_PARITY_MVN_ARGS
fi
set -- "$@" verify
ejc_run_cmd "$@"
