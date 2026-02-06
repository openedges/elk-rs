#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

SCENARIOS=${1:-issue_405,issue_603,issue_680,issue_871,issue_905}
ITERATIONS=${2:-20}
WARMUP=${3:-3}
OUTPUT=${4:-perf/java_results_layered_issue_scenarios.csv}

JAVA_PERF_MVN_BIN=${JAVA_PERF_MVN_BIN:-mvn}
JAVA_PERF_BUILD_PLUGINS=${JAVA_PERF_BUILD_PLUGINS:-true}
JAVA_PERF_EXTERNAL_ELK_ROOT=${JAVA_PERF_EXTERNAL_ELK_ROOT:-$REPO_ROOT/external/elk}
JAVA_PERF_EXTERNAL_ISOLATE=${JAVA_PERF_EXTERNAL_ISOLATE:-true}
JAVA_PERF_EXTERNAL_WORKTREE_ROOT=${JAVA_PERF_EXTERNAL_WORKTREE_ROOT:-${TMPDIR:-/tmp}}
JAVA_PERF_PREPARE_POM=${JAVA_PERF_PREPARE_POM:-}
JAVA_PERF_PREPARE_MODULES=${JAVA_PERF_PREPARE_MODULES:-}
JAVA_PERF_TEST_POM=${JAVA_PERF_TEST_POM:-}
JAVA_PERF_TEST_MODULES=${JAVA_PERF_TEST_MODULES:-../test/org.eclipse.elk.alg.test,../test/org.eclipse.elk.alg.layered.test}
JAVA_PERF_TEST_CLASS=${JAVA_PERF_TEST_CLASS:-LayeredIssuePerfBenchTest}
JAVA_PERF_TEST_METHOD=${JAVA_PERF_TEST_METHOD:-}
JAVA_PERF_TEST_GOAL=${JAVA_PERF_TEST_GOAL:-verify}
JAVA_PERF_BENCH_INJECT=${JAVA_PERF_BENCH_INJECT:-true}
JAVA_PERF_BENCH_SOURCE=${JAVA_PERF_BENCH_SOURCE:-$SCRIPT_DIR/java/LayeredIssuePerfBenchTest.java}
JAVA_PERF_BENCH_DEST=${JAVA_PERF_BENCH_DEST:-}
JAVA_PERF_BENCH_CLEANUP=${JAVA_PERF_BENCH_CLEANUP:-true}
JAVA_PERF_PREPARE_ARGS=${JAVA_PERF_PREPARE_ARGS:--DskipTests -DskipITs}
JAVA_PERF_DRY_RUN=${JAVA_PERF_DRY_RUN:-false}
JAVA_PERF_MVN_ARGS=${JAVA_PERF_MVN_ARGS:-}
JAVA_PERF_MVN_LOCAL_REPO=${JAVA_PERF_MVN_LOCAL_REPO:-}
JAVA_PERF_RETRIES=${JAVA_PERF_RETRIES:-0}
JAVA_PERF_RETRY_DELAY_SECS=${JAVA_PERF_RETRY_DELAY_SECS:-3}
JAVA_PERF_SKIP_DNS_CHECK=${JAVA_PERF_SKIP_DNS_CHECK:-false}
JAVA_PERF_REQUIRED_HOSTS=${JAVA_PERF_REQUIRED_HOSTS:-repo.eclipse.org,repo.maven.apache.org}

ELK_ROOT=$JAVA_PERF_EXTERNAL_ELK_ROOT
isolated_worktree_dir=
isolated_copy_dir=
isolation_mode=none
injected_bench_file=false

cleanup_injected_bench_file() {
  if [ "$injected_bench_file" = "true" ] && [ "$JAVA_PERF_BENCH_CLEANUP" = "true" ]; then
    rm -f "$JAVA_PERF_BENCH_DEST"
  fi
  if [ "$isolation_mode" = "worktree" ] && [ -n "$isolated_worktree_dir" ]; then
    git -C "$JAVA_PERF_EXTERNAL_ELK_ROOT" worktree remove --force "$isolated_worktree_dir" >/dev/null 2>&1 || true
  fi
  if [ -n "$isolated_worktree_dir" ]; then
    rm -rf "$isolated_worktree_dir"
  fi
  if [ -n "$isolated_copy_dir" ]; then
    rm -rf "$isolated_copy_dir"
  fi
}

trap cleanup_injected_bench_file EXIT

case "$JAVA_PERF_RETRIES" in
  ''|*[!0-9]*)
    echo "invalid JAVA_PERF_RETRIES (must be non-negative integer): $JAVA_PERF_RETRIES" >&2
    exit 1
    ;;
esac
case "$JAVA_PERF_RETRY_DELAY_SECS" in
  ''|*[!0-9]*)
    echo "invalid JAVA_PERF_RETRY_DELAY_SECS (must be non-negative integer): $JAVA_PERF_RETRY_DELAY_SECS" >&2
    exit 1
    ;;
esac

if [ "$JAVA_PERF_DRY_RUN" != "true" ]; then
  case "$JAVA_PERF_MVN_BIN" in
    */*)
      if [ ! -x "$JAVA_PERF_MVN_BIN" ]; then
        echo "maven command is not executable: $JAVA_PERF_MVN_BIN" >&2
        exit 1
      fi
      ;;
    *)
      if ! command -v "$JAVA_PERF_MVN_BIN" >/dev/null 2>&1; then
        echo "missing maven command in PATH: $JAVA_PERF_MVN_BIN" >&2
        exit 1
      fi
      ;;
  esac
fi

case "$OUTPUT" in
  /*)
    ;;
  *)
    OUTPUT="$(pwd)/$OUTPUT"
    ;;
esac

if [ "$JAVA_PERF_DRY_RUN" != "true" ] && [ "$JAVA_PERF_EXTERNAL_ISOLATE" = "true" ]; then
  isolated_worktree_dir=$(mktemp -d "$JAVA_PERF_EXTERNAL_WORKTREE_ROOT/elk-java-perf-worktree.XXXXXX")
  if [ -d "$isolated_worktree_dir" ]; then
    rmdir "$isolated_worktree_dir"
  fi
  if git -C "$JAVA_PERF_EXTERNAL_ELK_ROOT" worktree add --detach "$isolated_worktree_dir" HEAD >/dev/null 2>&1; then
    ELK_ROOT=$isolated_worktree_dir
    isolation_mode=worktree
  else
    rm -rf "$isolated_worktree_dir"
    isolated_worktree_dir=
    isolated_copy_dir=$(mktemp -d "$JAVA_PERF_EXTERNAL_WORKTREE_ROOT/elk-java-perf-copy.XXXXXX")
    cp -R "$JAVA_PERF_EXTERNAL_ELK_ROOT"/. "$isolated_copy_dir"/
    ELK_ROOT=$isolated_copy_dir
    isolation_mode=copy
    echo "warning: failed to create git worktree; using copied external/elk tree at: $isolated_copy_dir" >&2
  fi
fi

if [ -z "$JAVA_PERF_PREPARE_POM" ]; then
  JAVA_PERF_PREPARE_POM="$ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PERF_TEST_POM" ]; then
  JAVA_PERF_TEST_POM="$ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PERF_BENCH_DEST" ]; then
  JAVA_PERF_BENCH_DEST="$ELK_ROOT/test/org.eclipse.elk.alg.layered.test/src/org/eclipse/elk/alg/layered/issues/LayeredIssuePerfBenchTest.java"
fi

if [ "$JAVA_PERF_DRY_RUN" != "true" ] && [ "$JAVA_PERF_BENCH_INJECT" = "true" ]; then
  case "$JAVA_PERF_TEST_CLASS" in
    *LayeredIssuePerfBenchTest)
      if [ ! -f "$JAVA_PERF_BENCH_DEST" ]; then
        if [ ! -f "$JAVA_PERF_BENCH_SOURCE" ]; then
          echo "missing java bench source template: $JAVA_PERF_BENCH_SOURCE" >&2
          exit 1
        fi
        mkdir -p "$(dirname "$JAVA_PERF_BENCH_DEST")"
        cp "$JAVA_PERF_BENCH_SOURCE" "$JAVA_PERF_BENCH_DEST"
        injected_bench_file=true
      fi
      ;;
  esac
fi

can_resolve_host() {
  host_name=$1

  if command -v getent >/dev/null 2>&1; then
    if getent hosts "$host_name" >/dev/null 2>&1; then
      return 0
    fi
  fi

  if command -v dscacheutil >/dev/null 2>&1; then
    if dscacheutil -q host -a name "$host_name" 2>/dev/null | grep -q '^ip_address:'; then
      return 0
    fi
  fi

  if command -v dig >/dev/null 2>&1; then
    if [ -n "$(dig +short "$host_name" 2>/dev/null | awk 'NF { print; exit }')" ]; then
      return 0
    fi
  fi

  if command -v nslookup >/dev/null 2>&1; then
    if nslookup "$host_name" >/dev/null 2>&1; then
      return 0
    fi
  fi

  return 1
}

if [ "$JAVA_PERF_DRY_RUN" != "true" ] && [ "$JAVA_PERF_SKIP_DNS_CHECK" != "true" ]; then
  unresolved_hosts=""
  OLD_IFS=$IFS
  IFS=','
  # shellcheck disable=SC2086
  set -- $JAVA_PERF_REQUIRED_HOSTS
  IFS=$OLD_IFS
  for host_name in "$@"; do
    trimmed_host=$(printf '%s' "$host_name" | awk '{ gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print }')
    if [ -z "$trimmed_host" ]; then
      continue
    fi
    if ! can_resolve_host "$trimmed_host"; then
      if [ -n "$unresolved_hosts" ]; then
        unresolved_hosts="$unresolved_hosts,$trimmed_host"
      else
        unresolved_hosts="$trimmed_host"
      fi
    fi
  done

  if [ -n "$unresolved_hosts" ]; then
    echo "java perf dns preflight failed: unresolved hosts=$unresolved_hosts" >&2
    echo "hint: fix DNS/network access, provide reachable mirrors, or set JAVA_PERF_SKIP_DNS_CHECK=true to bypass preflight." >&2
    exit 1
  fi
fi

mkdir -p "$(dirname "$OUTPUT")"

run_cmd() {
  if [ "$JAVA_PERF_DRY_RUN" = "true" ]; then
    printf "java perf dry-run:"
    for arg in "$@"; do
      printf " %s" "$arg"
    done
    printf "\n"
    return
  fi
  attempt=0
  max_attempts=$((JAVA_PERF_RETRIES + 1))
  while [ "$attempt" -lt "$max_attempts" ]; do
    if "$@"; then
      return 0
    fi
    attempt=$((attempt + 1))
    if [ "$attempt" -lt "$max_attempts" ]; then
      echo "java perf command failed (attempt $attempt/$max_attempts); retrying in ${JAVA_PERF_RETRY_DELAY_SECS}s..." >&2
      if [ "$JAVA_PERF_RETRY_DELAY_SECS" -gt 0 ]; then
        sleep "$JAVA_PERF_RETRY_DELAY_SECS"
      fi
    fi
  done

  echo "java perf command failed after $max_attempts attempt(s)." >&2
  echo "hint: use JAVA_PERF_MVN_LOCAL_REPO to isolate Tycho metadata locks, or JAVA_PERF_DRY_RUN=true for rehearsal." >&2
  echo "hint: if dependency resolution fails, pre-warm Maven/Tycho artifacts or run with network access." >&2
  return 1
}

if [ "$JAVA_PERF_BUILD_PLUGINS" = "true" ]; then
  set -- "$JAVA_PERF_MVN_BIN" -f "$JAVA_PERF_PREPARE_POM"
  if [ -n "$JAVA_PERF_PREPARE_MODULES" ]; then
    set -- "$@" -pl "$JAVA_PERF_PREPARE_MODULES" -am
  fi
  if [ -n "$JAVA_PERF_MVN_LOCAL_REPO" ]; then
    set -- "$@" "-Dmaven.repo.local=$JAVA_PERF_MVN_LOCAL_REPO"
  fi
  if [ -n "$JAVA_PERF_PREPARE_ARGS" ]; then
    # Intentionally split on spaces so callers can pass additional maven flags.
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_PERF_PREPARE_ARGS
  fi
  if [ -n "$JAVA_PERF_MVN_ARGS" ]; then
    # Intentionally split on spaces so callers can pass additional maven flags.
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_PERF_MVN_ARGS
  fi
  set -- "$@" install
  run_cmd "$@"
fi

TEST_SELECTOR="$JAVA_PERF_TEST_CLASS"
if [ -n "$JAVA_PERF_TEST_METHOD" ]; then
  TEST_SELECTOR="$TEST_SELECTOR#$JAVA_PERF_TEST_METHOD"
fi
TYCHO_TEST_ARG_LINE="-Delk.perf.run=true -Delk.perf.scenarios=$SCENARIOS -Delk.perf.iterations=$ITERATIONS -Delk.perf.warmup=$WARMUP -Delk.perf.output=$OUTPUT"
set -- \
  "$JAVA_PERF_MVN_BIN" \
  -f "$JAVA_PERF_TEST_POM" \
  -pl "$JAVA_PERF_TEST_MODULES" \
  -am \
  "-Dtest=$TEST_SELECTOR" \
  -DfailIfNoTests=false \
  "-Dtycho.testArgLine=$TYCHO_TEST_ARG_LINE"

if [ -n "$JAVA_PERF_MVN_LOCAL_REPO" ]; then
  set -- "$@" "-Dmaven.repo.local=$JAVA_PERF_MVN_LOCAL_REPO"
fi
if [ -n "$JAVA_PERF_MVN_ARGS" ]; then
  # Intentionally split on spaces so callers can pass additional maven flags.
  # shellcheck disable=SC2086
  set -- "$@" $JAVA_PERF_MVN_ARGS
fi
set -- "$@" "$JAVA_PERF_TEST_GOAL"
run_cmd "$@"
