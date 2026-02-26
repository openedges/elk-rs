#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

SCENARIOS=${1:-issue_405,issue_603,issue_680,issue_871,issue_905}
ITERATIONS=${2:-20}
WARMUP=${3:-3}
OUTPUT=${4:-parity/java_results_layered_issue_scenarios.csv}

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
JAVA_PARITY_MVN_LOCAL_REPO=${JAVA_PARITY_MVN_LOCAL_REPO:-}
JAVA_PARITY_RETRIES=${JAVA_PARITY_RETRIES:-0}
JAVA_PARITY_RETRY_DELAY_SECS=${JAVA_PARITY_RETRY_DELAY_SECS:-3}
JAVA_PARITY_SKIP_DNS_CHECK=${JAVA_PARITY_SKIP_DNS_CHECK:-false}
JAVA_PARITY_REQUIRED_HOSTS=${JAVA_PARITY_REQUIRED_HOSTS:-repo.eclipse.org,repo.maven.apache.org}

ELK_ROOT=$JAVA_PARITY_EXTERNAL_ELK_ROOT
isolated_worktree_dir=
isolated_copy_dir=
isolation_mode=none
injected_bench_file=false

cleanup_injected_bench_file() {
  if [ "$injected_bench_file" = "true" ] && [ "$JAVA_PARITY_BENCH_CLEANUP" = "true" ]; then
    rm -f "$JAVA_PARITY_BENCH_DEST"
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

trap cleanup_injected_bench_file EXIT

case "$JAVA_PARITY_RETRIES" in
  ''|*[!0-9]*)
    echo "invalid JAVA_PARITY_RETRIES (must be non-negative integer): $JAVA_PARITY_RETRIES" >&2
    exit 1
    ;;
esac
case "$JAVA_PARITY_RETRY_DELAY_SECS" in
  ''|*[!0-9]*)
    echo "invalid JAVA_PARITY_RETRY_DELAY_SECS (must be non-negative integer): $JAVA_PARITY_RETRY_DELAY_SECS" >&2
    exit 1
    ;;
esac

if [ "$JAVA_PARITY_DRY_RUN" != "true" ]; then
  case "$JAVA_PARITY_MVN_BIN" in
    */*)
      if [ ! -x "$JAVA_PARITY_MVN_BIN" ]; then
        echo "maven command is not executable: $JAVA_PARITY_MVN_BIN" >&2
        exit 1
      fi
      ;;
    *)
      if ! command -v "$JAVA_PARITY_MVN_BIN" >/dev/null 2>&1; then
        echo "missing maven command in PATH: $JAVA_PARITY_MVN_BIN" >&2
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

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_EXTERNAL_ISOLATE" = "true" ]; then
  isolated_worktree_dir=$(mktemp -d "$JAVA_PARITY_EXTERNAL_WORKTREE_ROOT/elk-java-parity-worktree.XXXXXX")
  if [ -d "$isolated_worktree_dir" ]; then
    rmdir "$isolated_worktree_dir"
  fi
  if git -C "$JAVA_PARITY_EXTERNAL_ELK_ROOT" worktree add --detach "$isolated_worktree_dir" HEAD >/dev/null 2>&1; then
    ELK_ROOT=$isolated_worktree_dir
    isolation_mode=worktree
  else
    rm -rf "$isolated_worktree_dir"
    isolated_worktree_dir=
    isolated_copy_dir=$(mktemp -d "$JAVA_PARITY_EXTERNAL_WORKTREE_ROOT/elk-java-parity-copy.XXXXXX")
    cp -R "$JAVA_PARITY_EXTERNAL_ELK_ROOT"/. "$isolated_copy_dir"/
    ELK_ROOT=$isolated_copy_dir
    isolation_mode=copy
    echo "warning: failed to create git worktree; using copied external/elk tree at: $isolated_copy_dir" >&2
  fi
fi

if [ -z "$JAVA_PARITY_PREPARE_POM" ]; then
  JAVA_PARITY_PREPARE_POM="$ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_TEST_POM" ]; then
  JAVA_PARITY_TEST_POM="$ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_BENCH_DEST" ]; then
  JAVA_PARITY_BENCH_DEST="$ELK_ROOT/test/org.eclipse.elk.alg.layered.test/src/org/eclipse/elk/alg/layered/issues/LayeredIssueParityBenchTest.java"
fi

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_BENCH_INJECT" = "true" ]; then
  case "$JAVA_PARITY_TEST_CLASS" in
    *LayeredIssueParityBenchTest)
      if [ ! -f "$JAVA_PARITY_BENCH_DEST" ]; then
        if [ ! -f "$JAVA_PARITY_BENCH_SOURCE" ]; then
          echo "missing java bench source template: $JAVA_PARITY_BENCH_SOURCE" >&2
          exit 1
        fi
        mkdir -p "$(dirname "$JAVA_PARITY_BENCH_DEST")"
        cp "$JAVA_PARITY_BENCH_SOURCE" "$JAVA_PARITY_BENCH_DEST"
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

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_SKIP_DNS_CHECK" != "true" ]; then
  unresolved_hosts=""
  OLD_IFS=$IFS
  IFS=','
  # shellcheck disable=SC2086
  set -- $JAVA_PARITY_REQUIRED_HOSTS
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
    echo "java parity dns preflight failed: unresolved hosts=$unresolved_hosts" >&2
    echo "hint: fix DNS/network access, provide reachable mirrors, or set JAVA_PARITY_SKIP_DNS_CHECK=true to bypass preflight." >&2
    exit 1
  fi
fi

mkdir -p "$(dirname "$OUTPUT")"

run_cmd() {
  if [ "$JAVA_PARITY_DRY_RUN" = "true" ]; then
    printf "java parity dry-run:"
    for arg in "$@"; do
      printf " %s" "$arg"
    done
    printf "\n"
    return
  fi
  attempt=0
  max_attempts=$((JAVA_PARITY_RETRIES + 1))
  while [ "$attempt" -lt "$max_attempts" ]; do
    if "$@"; then
      return 0
    fi
    attempt=$((attempt + 1))
    if [ "$attempt" -lt "$max_attempts" ]; then
      echo "java parity command failed (attempt $attempt/$max_attempts); retrying in ${JAVA_PARITY_RETRY_DELAY_SECS}s..." >&2
      if [ "$JAVA_PARITY_RETRY_DELAY_SECS" -gt 0 ]; then
        sleep "$JAVA_PARITY_RETRY_DELAY_SECS"
      fi
    fi
  done

  echo "java parity command failed after $max_attempts attempt(s)." >&2
  echo "hint: use JAVA_PARITY_MVN_LOCAL_REPO to isolate Tycho metadata locks, or JAVA_PARITY_DRY_RUN=true for rehearsal." >&2
  echo "hint: if dependency resolution fails, pre-warm Maven/Tycho artifacts or run with network access." >&2
  return 1
}

if [ "$JAVA_PARITY_BUILD_PLUGINS" = "true" ]; then
  set -- "$JAVA_PARITY_MVN_BIN" -f "$JAVA_PARITY_PREPARE_POM"
  if [ -n "$JAVA_PARITY_PREPARE_MODULES" ]; then
    set -- "$@" -pl "$JAVA_PARITY_PREPARE_MODULES" -am
  fi
  if [ -n "$JAVA_PARITY_MVN_LOCAL_REPO" ]; then
    set -- "$@" "-Dmaven.repo.local=$JAVA_PARITY_MVN_LOCAL_REPO"
  fi
  if [ -n "$JAVA_PARITY_PREPARE_ARGS" ]; then
    # Intentionally split on spaces so callers can pass additional maven flags.
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_PARITY_PREPARE_ARGS
  fi
  if [ -n "$JAVA_PARITY_MVN_ARGS" ]; then
    # Intentionally split on spaces so callers can pass additional maven flags.
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_PARITY_MVN_ARGS
  fi
  set -- "$@" install
  run_cmd "$@"
fi

TEST_SELECTOR="$JAVA_PARITY_TEST_CLASS"
if [ -n "$JAVA_PARITY_TEST_METHOD" ]; then
  TEST_SELECTOR="$TEST_SELECTOR#$JAVA_PARITY_TEST_METHOD"
fi
TYCHO_TEST_ARG_LINE="-Delk.parity.run=true -Delk.parity.scenarios=$SCENARIOS -Delk.parity.iterations=$ITERATIONS -Delk.parity.warmup=$WARMUP -Delk.parity.output=$OUTPUT"
set -- \
  "$JAVA_PARITY_MVN_BIN" \
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
  # Intentionally split on spaces so callers can pass additional maven flags.
  # shellcheck disable=SC2086
  set -- "$@" $JAVA_PARITY_MVN_ARGS
fi
set -- "$@" "$JAVA_PARITY_TEST_GOAL"
run_cmd "$@"
