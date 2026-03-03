#!/bin/sh
# ============================================================================
# java_model_parity_trace.sh — Java model parity export with determinism patches
#
# Injects ElkModelParityExportTest.java into external ELK, applies determinism
# patches, purges stale SNAPSHOT caches, builds ELK plugins, and runs the
# parity export test via Tycho.
#
# Usage:
#   sh scripts/java_model_parity_trace.sh [models_root] [output_dir]
#
# Environment variables: all JAVA_PARITY_* vars from the original
# java_model_parity_trace.sh are supported.  See scripts/README.md.
# ============================================================================
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

EJC_PREFIX=JAVA_PARITY
# shellcheck source=java/elk_java_common.sh
. "$SCRIPT_DIR/java/elk_java_common.sh"

# ========================== Configuration ===================================

MODELS_ROOT_INPUT=${1:-external/elk-models}
OUTPUT_DIR_INPUT=${2:-tests/model_parity/java}

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

# Resolve input paths to absolute
ejc_resolve_to_absolute "$MODELS_ROOT_INPUT" "$REPO_ROOT"; MODELS_ROOT=$_ejc_val
ejc_resolve_to_absolute "$OUTPUT_DIR_INPUT" "$REPO_ROOT";  OUTPUT_DIR=$_ejc_val

# ========================== Script-specific state ===========================

EJC_ELK_ROOT=$JAVA_PARITY_EXTERNAL_ELK_ROOT
manifest_backup=
manifest_was_present=false
manifest_patched=false

# ========================== Cleanup =========================================

_parity_cleanup() {
  # Restore MANIFEST.MF
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

  # Restore injected Java file
  ejc_restore_java_file
}

ejc_register_cleanup _parity_cleanup

# ========================== Validation ======================================

ejc_validate_integer JAVA_PARITY_RETRIES "$JAVA_PARITY_RETRIES"
ejc_validate_integer JAVA_PARITY_RETRY_DELAY_SECS "$JAVA_PARITY_RETRY_DELAY_SECS"
ejc_validate_maven

# ========================== Clean check + Isolation =========================

ejc_check_clean_elk
ejc_create_isolation parity

# ========================== Patches + SNAPSHOT purge ========================

ejc_apply_patches
ejc_purge_snapshot_cache

# ========================== Resolve ELK-relative paths ======================

if [ -z "$JAVA_PARITY_PREPARE_POM" ]; then
  JAVA_PARITY_PREPARE_POM="$EJC_ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_TEST_POM" ]; then
  JAVA_PARITY_TEST_POM="$EJC_ELK_ROOT/build/pom.xml"
fi
if [ -z "$JAVA_PARITY_BENCH_DEST" ]; then
  JAVA_PARITY_BENCH_DEST="$EJC_ELK_ROOT/test/org.eclipse.elk.graph.json.test/src/org/eclipse/elk/graph/json/test/ElkModelParityExportTest.java"
fi
if [ -z "$JAVA_PARITY_TEST_MANIFEST" ]; then
  JAVA_PARITY_TEST_MANIFEST="$EJC_ELK_ROOT/test/org.eclipse.elk.graph.json.test/META-INF/MANIFEST.MF"
fi

# ========================== Inject test class ===============================

ejc_inject_java_file "$JAVA_PARITY_BENCH_SOURCE" "$JAVA_PARITY_BENCH_DEST"

# ========================== Patch MANIFEST.MF ===============================

if [ "$JAVA_PARITY_DRY_RUN" != "true" ] && [ "$JAVA_PARITY_PATCH_TEST_MANIFEST" = "true" ] && [ -f "$JAVA_PARITY_TEST_MANIFEST" ]; then
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

# ========================== Build plugins ===================================

ejc_mvn_build_plugins "$JAVA_PARITY_PREPARE_POM"

# ========================== Build exclude tokens ============================

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

# ========================== Run Tycho test ==================================

TYCHO_TEST_ARG_LINE="-Delk.parity.run=true -Delk.parity.modelsRoot=$MODELS_ROOT -Delk.parity.outputDir=$OUTPUT_DIR -Delk.parity.limit=$JAVA_PARITY_LIMIT -Delk.parity.include=$JAVA_PARITY_INCLUDE -Delk.parity.exclude=$JAVA_PARITY_EXCLUDE_TOKENS -Delk.parity.failFast=$JAVA_PARITY_FAIL_FAST -Delk.parity.prettyPrint=$JAVA_PARITY_PRETTY_PRINT -Delk.parity.resetOutput=$JAVA_PARITY_RESET_OUTPUT -Delk.parity.randomSeed=$JAVA_PARITY_RANDOM_SEED"

ejc_resolve_var MVN_BIN mvn; _mvn=$_ejc_val
set -- "$_mvn" -f "$JAVA_PARITY_TEST_POM"
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
ejc_run_cmd "$@"

echo "java model parity export finished: output=$OUTPUT_DIR manifest=$OUTPUT_DIR/java_manifest.tsv"
