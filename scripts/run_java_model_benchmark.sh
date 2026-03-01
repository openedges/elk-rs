#!/bin/sh
# Java model benchmark runner for 5-way performance comparison.
#
# Follows the same isolation/injection pattern as run_java_parity_layered_issue_scenarios.sh
# but targets ElkModelBenchTest (JSON-based benchmark via RecursiveGraphLayoutEngine).
#
# Usage:
#   sh scripts/run_java_model_benchmark.sh [mode] [iterations] [warmup] [output]
#
# Environment variables (override defaults):
#   JAVA_BENCH_MODE          — synthetic or models (default: models)
#   JAVA_BENCH_MANIFEST      — manifest TSV path for models mode
#   JAVA_BENCH_ITERATIONS    — iterations per scenario (default: 20)
#   JAVA_BENCH_WARMUP        — warmup iterations (default: 3)
#   JAVA_BENCH_OUTPUT        — CSV output path
#   JAVA_BENCH_LIMIT         — max models (default: 50)
#
# All JAVA_PARITY_* env vars from run_java_parity_layered_issue_scenarios.sh
# are also supported for Maven/isolation configuration.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

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

ELK_ROOT=$JAVA_PARITY_EXTERNAL_ELK_ROOT
isolated_worktree_dir=
isolated_copy_dir=
isolation_mode=none
injected_bench_file=false

BENCH_SOURCE="$SCRIPT_DIR/java/ElkModelBenchTest.java"
BENCH_DEST=

cleanup() {
  if [ "$injected_bench_file" = "true" ] && [ -n "$BENCH_DEST" ]; then
    rm -f "$BENCH_DEST"
  fi
  if [ "$isolation_mode" = "worktree" ] && [ -n "$isolated_worktree_dir" ]; then
    git -C "$JAVA_PARITY_EXTERNAL_ELK_ROOT" worktree remove --force "$isolated_worktree_dir" >/dev/null 2>&1 || true
  fi
  if [ -n "$isolated_worktree_dir" ]; then
    rm -rf "$isolated_worktree_dir"
  fi
  if [ -n "$isolated_copy_dir" ]; then
    rm -rf "$isolated_copy_dir"
  fi
}

trap cleanup EXIT

# Validate retries
case "$JAVA_PARITY_RETRIES" in
  ''|*[!0-9]*)
    echo "invalid JAVA_PARITY_RETRIES: $JAVA_PARITY_RETRIES" >&2
    exit 1
    ;;
esac

# Validate maven
if [ "$JAVA_PARITY_DRY_RUN" != "true" ]; then
  case "$JAVA_PARITY_MVN_BIN" in
    */*)
      if [ ! -x "$JAVA_PARITY_MVN_BIN" ]; then
        echo "maven not executable: $JAVA_PARITY_MVN_BIN" >&2
        exit 1
      fi
      ;;
    *)
      if ! command -v "$JAVA_PARITY_MVN_BIN" >/dev/null 2>&1; then
        echo "maven not in PATH: $JAVA_PARITY_MVN_BIN" >&2
        exit 1
      fi
      ;;
  esac
fi

# Resolve output to absolute path
case "$OUTPUT" in
  /*) ;;
  *) OUTPUT="$(pwd)/$OUTPUT" ;;
esac

# Resolve manifest to absolute path
case "$JAVA_BENCH_MANIFEST" in
  /*) ;;
  *) JAVA_BENCH_MANIFEST="$(pwd)/$JAVA_BENCH_MANIFEST" ;;
esac

# Isolation: create worktree or copy
if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_EXTERNAL_ISOLATE" = "true" ]; then
  isolated_worktree_dir=$(mktemp -d "$JAVA_PARITY_EXTERNAL_WORKTREE_ROOT/elk-java-bench-worktree.XXXXXX")
  if [ -d "$isolated_worktree_dir" ]; then
    rmdir "$isolated_worktree_dir"
  fi
  if git -C "$JAVA_PARITY_EXTERNAL_ELK_ROOT" worktree add --detach "$isolated_worktree_dir" HEAD >/dev/null 2>&1; then
    ELK_ROOT=$isolated_worktree_dir
    isolation_mode=worktree
  else
    rm -rf "$isolated_worktree_dir"
    isolated_worktree_dir=
    isolated_copy_dir=$(mktemp -d "$JAVA_PARITY_EXTERNAL_WORKTREE_ROOT/elk-java-bench-copy.XXXXXX")
    cp -R "$JAVA_PARITY_EXTERNAL_ELK_ROOT"/. "$isolated_copy_dir"/
    ELK_ROOT=$isolated_copy_dir
    isolation_mode=copy
    echo "warning: worktree failed; using copy at: $isolated_copy_dir" >&2
  fi
fi

# Set default paths relative to ELK_ROOT
PREPARE_POM="$ELK_ROOT/build/pom.xml"
TEST_POM="$ELK_ROOT/build/pom.xml"
TEST_MODULES="../test/org.eclipse.elk.graph.json.test"
BENCH_DEST="$ELK_ROOT/test/org.eclipse.elk.graph.json.test/src/org/eclipse/elk/graph/json/test/ElkModelBenchTest.java"

# Inject bench test class
if [ "$JAVA_PARITY_DRY_RUN" != "true" ]; then
  if [ ! -f "$BENCH_DEST" ]; then
    if [ ! -f "$BENCH_SOURCE" ]; then
      echo "missing bench source: $BENCH_SOURCE" >&2
      exit 1
    fi
    mkdir -p "$(dirname "$BENCH_DEST")"
    cp "$BENCH_SOURCE" "$BENCH_DEST"
    injected_bench_file=true
  fi
fi

# DNS preflight (reuse from layered runner)
can_resolve_host() {
  host_name=$1
  if command -v getent >/dev/null 2>&1; then
    if getent hosts "$host_name" >/dev/null 2>&1; then return 0; fi
  fi
  if command -v dig >/dev/null 2>&1; then
    if [ -n "$(dig +short "$host_name" 2>/dev/null | awk 'NF { print; exit }')" ]; then return 0; fi
  fi
  if command -v nslookup >/dev/null 2>&1; then
    if nslookup "$host_name" >/dev/null 2>&1; then return 0; fi
  fi
  return 1
}

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_SKIP_DNS_CHECK" != "true" ]; then
  unresolved=""
  OLD_IFS=$IFS
  IFS=','
  # shellcheck disable=SC2086
  set -- $JAVA_PARITY_REQUIRED_HOSTS
  IFS=$OLD_IFS
  for h in "$@"; do
    trimmed=$(printf '%s' "$h" | awk '{ gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print }')
    [ -z "$trimmed" ] && continue
    if ! can_resolve_host "$trimmed"; then
      unresolved="${unresolved:+$unresolved,}$trimmed"
    fi
  done
  if [ -n "$unresolved" ]; then
    echo "DNS preflight failed: $unresolved" >&2
    exit 1
  fi
fi

mkdir -p "$(dirname "$OUTPUT")"

# Retryable command runner
run_cmd() {
  if [ "$JAVA_PARITY_DRY_RUN" = "true" ]; then
    printf "dry-run:"; for arg in "$@"; do printf " %s" "$arg"; done; printf "\n"
    return
  fi
  attempt=0
  max_attempts=$((JAVA_PARITY_RETRIES + 1))
  while [ "$attempt" -lt "$max_attempts" ]; do
    if "$@"; then return 0; fi
    attempt=$((attempt + 1))
    if [ "$attempt" -lt "$max_attempts" ]; then
      echo "command failed (attempt $attempt/$max_attempts); retrying in ${JAVA_PARITY_RETRY_DELAY_SECS}s..." >&2
      [ "$JAVA_PARITY_RETRY_DELAY_SECS" -gt 0 ] && sleep "$JAVA_PARITY_RETRY_DELAY_SECS"
    fi
  done
  echo "command failed after $max_attempts attempt(s)." >&2
  return 1
}

# Build plugins if needed
if [ "$JAVA_PARITY_BUILD_PLUGINS" = "true" ]; then
  set -- "$JAVA_PARITY_MVN_BIN" -f "$PREPARE_POM"
  if [ -n "$JAVA_PARITY_MVN_LOCAL_REPO" ]; then
    set -- "$@" "-Dmaven.repo.local=$JAVA_PARITY_MVN_LOCAL_REPO"
  fi
  if [ -n "$JAVA_PARITY_PREPARE_ARGS" ]; then
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_PARITY_PREPARE_ARGS
  fi
  if [ -n "$JAVA_PARITY_MVN_ARGS" ]; then
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_PARITY_MVN_ARGS
  fi
  set -- "$@" install
  run_cmd "$@"
fi

# Run benchmark test
TYCHO_TEST_ARG_LINE="-Delk.parity.run=true -Delk.bench.mode=$MODE -Delk.bench.iterations=$ITERATIONS -Delk.bench.warmup=$WARMUP -Delk.bench.output=$OUTPUT -Delk.bench.manifest=$JAVA_BENCH_MANIFEST -Delk.bench.limit=$JAVA_BENCH_LIMIT"
set -- \
  "$JAVA_PARITY_MVN_BIN" \
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
run_cmd "$@"
