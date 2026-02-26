#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

MODELS_ROOT_INPUT=${1:-external/elk-models}
OUTPUT_DIR_INPUT=${2:-parity/model_parity/java}

JAVA_PARITY_MVN_BIN=${JAVA_PARITY_MVN_BIN:-mvn}
JAVA_PARITY_BUILD_PLUGINS=${JAVA_PARITY_BUILD_PLUGINS:-true}
JAVA_PARITY_EXTERNAL_ELK_ROOT=${JAVA_PARITY_EXTERNAL_ELK_ROOT:-$REPO_ROOT/external/elk}
JAVA_PARITY_EXTERNAL_ISOLATE=${JAVA_PARITY_EXTERNAL_ISOLATE:-true}
JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=${JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK:-true}
JAVA_PARITY_EXTERNAL_WORKTREE_ROOT=${JAVA_PARITY_EXTERNAL_WORKTREE_ROOT:-${TMPDIR:-/tmp}}
JAVA_PARITY_PREPARE_POM=${JAVA_PARITY_PREPARE_POM:-}
JAVA_PARITY_TEST_POM=${JAVA_PARITY_TEST_POM:-}
JAVA_PARITY_PREPARE_MODULES=${JAVA_PARITY_PREPARE_MODULES:-}
JAVA_PARITY_TEST_MODULES=${JAVA_PARITY_TEST_MODULES:-../test/org.eclipse.elk.alg.test,../test/org.eclipse.elk.graph.json.test}
JAVA_PARITY_TEST_CLASS=${JAVA_PARITY_TEST_CLASS:-org.eclipse.elk.graph.json.test.ElkModelParityExportTest}
JAVA_PARITY_TEST_GOAL=${JAVA_PARITY_TEST_GOAL:-verify}
JAVA_PARITY_BENCH_SOURCE=${JAVA_PARITY_BENCH_SOURCE:-$SCRIPT_DIR/java/ElkModelParityExportTest.java}
JAVA_PARITY_BENCH_DEST=${JAVA_PARITY_BENCH_DEST:-}
JAVA_PARITY_TEST_MANIFEST=${JAVA_PARITY_TEST_MANIFEST:-}
JAVA_PARITY_PATCH_TEST_MANIFEST=${JAVA_PARITY_PATCH_TEST_MANIFEST:-true}
JAVA_PARITY_BENCH_CLEANUP=${JAVA_PARITY_BENCH_CLEANUP:-true}
JAVA_PARITY_PREPARE_ARGS=${JAVA_PARITY_PREPARE_ARGS:--DskipTests -DskipITs}
JAVA_PARITY_MVN_LOCAL_REPO=${JAVA_PARITY_MVN_LOCAL_REPO:-}
JAVA_PARITY_MVN_ARGS=${JAVA_PARITY_MVN_ARGS:-}
JAVA_PARITY_RETRIES=${JAVA_PARITY_RETRIES:-0}
JAVA_PARITY_RETRY_DELAY_SECS=${JAVA_PARITY_RETRY_DELAY_SECS:-3}
JAVA_PARITY_DRY_RUN=${JAVA_PARITY_DRY_RUN:-false}

JAVA_PARITY_PATCHES_DIR=${JAVA_PARITY_PATCHES_DIR:-$REPO_ROOT/scripts/java/patches}
JAVA_PARITY_APPLY_PATCHES=${JAVA_PARITY_APPLY_PATCHES:-true}

JAVA_PARITY_LIMIT=${JAVA_PARITY_LIMIT:-0}
JAVA_PARITY_INCLUDE=${JAVA_PARITY_INCLUDE:-}
JAVA_PARITY_EXCLUDE=${JAVA_PARITY_EXCLUDE:-}
JAVA_PARITY_EXCLUDE_FILE=${JAVA_PARITY_EXCLUDE_FILE:-}
JAVA_PARITY_FAIL_FAST=${JAVA_PARITY_FAIL_FAST:-false}
JAVA_PARITY_PRETTY_PRINT=${JAVA_PARITY_PRETTY_PRINT:-false}
JAVA_PARITY_RESET_OUTPUT=${JAVA_PARITY_RESET_OUTPUT:-true}
JAVA_PARITY_RANDOM_SEED=${JAVA_PARITY_RANDOM_SEED:-1}

MODELS_ROOT=$MODELS_ROOT_INPUT
OUTPUT_DIR=$OUTPUT_DIR_INPUT

case "$MODELS_ROOT" in
  /*) ;;
  *) MODELS_ROOT="$REPO_ROOT/$MODELS_ROOT" ;;
esac
case "$OUTPUT_DIR" in
  /*) ;;
  *) OUTPUT_DIR="$REPO_ROOT/$OUTPUT_DIR" ;;
esac

ELK_ROOT=$JAVA_PARITY_EXTERNAL_ELK_ROOT
isolation_mode=none
isolated_worktree_dir=
isolated_copy_dir=
bench_backup=
bench_was_present=false
bench_written=false
manifest_backup=
manifest_was_present=false
manifest_patched=false

cleanup() {
  if [ "$manifest_patched" = "true" ] && [ "$JAVA_PARITY_BENCH_CLEANUP" = "true" ]; then
    if [ -n "$manifest_backup" ] && [ -f "$manifest_backup" ]; then
      cp "$manifest_backup" "$JAVA_PARITY_TEST_MANIFEST"
    elif [ "$manifest_was_present" = "false" ]; then
      rm -f "$JAVA_PARITY_TEST_MANIFEST"
    fi
  fi

  if [ -n "$manifest_backup" ] && [ -f "$manifest_backup" ]; then
    rm -f "$manifest_backup"
  fi

  if [ "$bench_written" = "true" ] && [ "$JAVA_PARITY_BENCH_CLEANUP" = "true" ]; then
    if [ -n "$bench_backup" ] && [ -f "$bench_backup" ]; then
      cp "$bench_backup" "$JAVA_PARITY_BENCH_DEST"
    elif [ "$bench_was_present" = "false" ]; then
      rm -f "$JAVA_PARITY_BENCH_DEST"
    fi
  fi

  if [ -n "$bench_backup" ] && [ -f "$bench_backup" ]; then
    rm -f "$bench_backup"
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

trap cleanup EXIT INT TERM

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

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK" = "true" ]; then
  if ! git -C "$JAVA_PARITY_EXTERNAL_ELK_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=true but external ELK root is not a git worktree: $JAVA_PARITY_EXTERNAL_ELK_ROOT" >&2
    echo "set JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=false to bypass this guard." >&2
    exit 1
  fi

  dirty_status=$(git -C "$JAVA_PARITY_EXTERNAL_ELK_ROOT" status --porcelain 2>/dev/null || true)
  if [ -n "$dirty_status" ]; then
    echo "external ELK tree has local changes; refusing parity export to protect external/elk state." >&2
    printf "%s\n" "$dirty_status" | sed -n '1,20p' >&2
    dirty_lines=$(printf "%s\n" "$dirty_status" | wc -l | awk '{print $1}')
    if [ "$dirty_lines" -gt 20 ]; then
      echo "... (showing first 20 of $dirty_lines changed paths)" >&2
    fi
    echo "set JAVA_PARITY_REQUIRE_CLEAN_EXTERNAL_ELK=false to bypass this guard." >&2
    exit 1
  fi
fi

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

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_APPLY_PATCHES" = "true" ] && [ -d "$JAVA_PARITY_PATCHES_DIR" ]; then
  for p in "$JAVA_PARITY_PATCHES_DIR"/*.patch; do
    [ -f "$p" ] || continue
    if git -C "$ELK_ROOT" apply "$p"; then
      echo "java parity: applied patch $(basename "$p")"
    else
      echo "java parity: failed to apply patch $(basename "$p")" >&2
      exit 1
    fi
  done
fi

if [ -z "$JAVA_PARITY_PREPARE_POM" ]; then
  JAVA_PARITY_PREPARE_POM="$ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_TEST_POM" ]; then
  JAVA_PARITY_TEST_POM="$ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_BENCH_DEST" ]; then
  JAVA_PARITY_BENCH_DEST="$ELK_ROOT/test/org.eclipse.elk.graph.json.test/src/org/eclipse/elk/graph/json/test/ElkModelParityExportTest.java"
fi
if [ -z "$JAVA_PARITY_TEST_MANIFEST" ]; then
  JAVA_PARITY_TEST_MANIFEST="$ELK_ROOT/test/org.eclipse.elk.graph.json.test/META-INF/MANIFEST.MF"
fi

if [ ! -f "$JAVA_PARITY_BENCH_SOURCE" ]; then
  echo "missing java parity test source: $JAVA_PARITY_BENCH_SOURCE" >&2
  exit 1
fi

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
  return 1
}

if [ "$JAVA_PARITY_DRY_RUN" != "true" ]; then
  mkdir -p "$(dirname "$JAVA_PARITY_BENCH_DEST")"
  if [ -f "$JAVA_PARITY_BENCH_DEST" ]; then
    bench_was_present=true
    bench_backup=$(mktemp "${TMPDIR:-/tmp}/elk-java-parity-bench-backup.XXXXXX")
    cp "$JAVA_PARITY_BENCH_DEST" "$bench_backup"
  fi
  cp "$JAVA_PARITY_BENCH_SOURCE" "$JAVA_PARITY_BENCH_DEST"
  bench_written=true

  if [ "$JAVA_PARITY_PATCH_TEST_MANIFEST" = "true" ] && [ -f "$JAVA_PARITY_TEST_MANIFEST" ]; then
    if ! grep -q "org.eclipse.emf.ecore" "$JAVA_PARITY_TEST_MANIFEST"; then
      manifest_was_present=true
      manifest_backup=$(mktemp "${TMPDIR:-/tmp}/elk-java-parity-manifest-backup.XXXXXX")
      cp "$JAVA_PARITY_TEST_MANIFEST" "$manifest_backup"
      perl -0pi -e 's/(org\.eclipse\.emf\.common,\n)/$1 org.eclipse.emf.ecore,\n/' "$JAVA_PARITY_TEST_MANIFEST"
      if ! grep -q "org.eclipse.emf.ecore" "$JAVA_PARITY_TEST_MANIFEST"; then
        echo "failed to patch test manifest for org.eclipse.emf.ecore dependency: $JAVA_PARITY_TEST_MANIFEST" >&2
        exit 1
      fi
      manifest_patched=true
    fi
  fi
fi

if [ "$JAVA_PARITY_BUILD_PLUGINS" = "true" ]; then
  set -- "$JAVA_PARITY_MVN_BIN" -f "$JAVA_PARITY_PREPARE_POM"
  if [ -n "$JAVA_PARITY_PREPARE_MODULES" ]; then
    set -- "$@" -pl "$JAVA_PARITY_PREPARE_MODULES" -am
  fi
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

JAVA_PARITY_EXCLUDE_TOKENS=$JAVA_PARITY_EXCLUDE
if [ -n "$JAVA_PARITY_EXCLUDE_FILE" ] && [ -f "$JAVA_PARITY_EXCLUDE_FILE" ]; then
  file_tokens=$(grep -v '^[[:space:]]*$' "$JAVA_PARITY_EXCLUDE_FILE" | grep -v '^[[:space:]]*#' | tr -d '\r' | tr '\n' ',' | sed 's/,$//')
  if [ -n "$file_tokens" ]; then
    if [ -n "$JAVA_PARITY_EXCLUDE_TOKENS" ]; then
      JAVA_PARITY_EXCLUDE_TOKENS="$JAVA_PARITY_EXCLUDE_TOKENS,$file_tokens"
    else
      JAVA_PARITY_EXCLUDE_TOKENS="$file_tokens"
    fi
  fi
fi

TYCHO_TEST_ARG_LINE="-Delk.parity.run=true -Delk.parity.modelsRoot=$MODELS_ROOT -Delk.parity.outputDir=$OUTPUT_DIR -Delk.parity.limit=$JAVA_PARITY_LIMIT -Delk.parity.include=$JAVA_PARITY_INCLUDE -Delk.parity.exclude=$JAVA_PARITY_EXCLUDE_TOKENS -Delk.parity.failFast=$JAVA_PARITY_FAIL_FAST -Delk.parity.prettyPrint=$JAVA_PARITY_PRETTY_PRINT -Delk.parity.resetOutput=$JAVA_PARITY_RESET_OUTPUT -Delk.parity.randomSeed=$JAVA_PARITY_RANDOM_SEED"

set -- "$JAVA_PARITY_MVN_BIN" -f "$JAVA_PARITY_TEST_POM"
if [ -n "$JAVA_PARITY_TEST_MODULES" ]; then
  set -- "$@" -pl "$JAVA_PARITY_TEST_MODULES" -am
fi
if [ -n "$JAVA_PARITY_MVN_LOCAL_REPO" ]; then
  set -- "$@" "-Dmaven.repo.local=$JAVA_PARITY_MVN_LOCAL_REPO"
fi
set -- "$@" "-Dtest=$JAVA_PARITY_TEST_CLASS"
set -- "$@" "-DfailIfNoTests=false"
set -- "$@" "-Dtycho.testArgLine=$TYCHO_TEST_ARG_LINE"
if [ -n "$JAVA_PARITY_MVN_ARGS" ]; then
  # shellcheck disable=SC2086
  set -- "$@" $JAVA_PARITY_MVN_ARGS
fi
set -- "$@" "$JAVA_PARITY_TEST_GOAL"
run_cmd "$@"

echo "java model parity export finished: output=$OUTPUT_DIR manifest=$OUTPUT_DIR/java_manifest.tsv"
