#!/bin/sh
# ============================================================================
# java_model_phase_step_trace.sh — Java ELK phase-step trace exporter
#
# Builds ELK plugins, compiles ElkPhaseTraceExporter.java against the built
# classpath, and runs it to export per-step JSON traces for each model.
#
# Now also applies determinism patches and purges stale SNAPSHOT caches
# (controlled by JAVA_TRACE_APPLY_PATCHES and JAVA_TRACE_PURGE_SNAPSHOTS).
#
# Usage:
#   sh scripts/java_model_phase_step_trace.sh [models_root] [output_dir]
#
# Environment variables: all JAVA_TRACE_* vars are supported.
# ============================================================================
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

EJC_PREFIX=JAVA_TRACE
# shellcheck source=java/elk_java_common.sh
. "$SCRIPT_DIR/java/elk_java_common.sh"

# ========================== Configuration ===================================

MODELS_ROOT_INPUT=${1:-external/elk-models}
OUTPUT_DIR_INPUT=${2:-tests/model_parity/java_trace}

JAVA_TRACE_MVN_BIN=${JAVA_TRACE_MVN_BIN:-mvn}
JAVA_TRACE_BUILD_PLUGINS=${JAVA_TRACE_BUILD_PLUGINS:-true}
JAVA_TRACE_EXTERNAL_ELK_ROOT=${JAVA_TRACE_EXTERNAL_ELK_ROOT:-$REPO_ROOT/external/elk}
JAVA_TRACE_EXTERNAL_ISOLATE=${JAVA_TRACE_EXTERNAL_ISOLATE:-true}
JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK=${JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK:-true}
JAVA_TRACE_EXTERNAL_WORKTREE_ROOT=${JAVA_TRACE_EXTERNAL_WORKTREE_ROOT:-${TMPDIR:-/tmp}}
JAVA_TRACE_PREPARE_ARGS=${JAVA_TRACE_PREPARE_ARGS:--DskipTests -DskipITs}
JAVA_TRACE_MVN_LOCAL_REPO=${JAVA_TRACE_MVN_LOCAL_REPO:-}
JAVA_TRACE_MVN_ARGS=${JAVA_TRACE_MVN_ARGS:-}
JAVA_TRACE_DRY_RUN=${JAVA_TRACE_DRY_RUN:-false}
JAVA_TRACE_RETRIES=${JAVA_TRACE_RETRIES:-0}
JAVA_TRACE_RETRY_DELAY_SECS=${JAVA_TRACE_RETRY_DELAY_SECS:-3}

# New: determinism patches and SNAPSHOT purge (default: enabled)
JAVA_TRACE_PATCHES_DIR=${JAVA_TRACE_PATCHES_DIR:-$REPO_ROOT/scripts/java/patches}
JAVA_TRACE_APPLY_PATCHES=${JAVA_TRACE_APPLY_PATCHES:-true}

JAVA_TRACE_BENCH_SOURCE=${JAVA_TRACE_BENCH_SOURCE:-$SCRIPT_DIR/java/ElkPhaseTraceExporter.java}

JAVA_TRACE_LIMIT=${JAVA_TRACE_LIMIT:-0}
JAVA_TRACE_INCLUDE=${JAVA_TRACE_INCLUDE:-}
JAVA_TRACE_EXCLUDE=${JAVA_TRACE_EXCLUDE:-}
JAVA_TRACE_RANDOM_SEED=${JAVA_TRACE_RANDOM_SEED:-1}
JAVA_TRACE_PRETTY_PRINT=${JAVA_TRACE_PRETTY_PRINT:-true}

# Resolve input paths to absolute
ejc_resolve_to_absolute "$MODELS_ROOT_INPUT" "$REPO_ROOT"; MODELS_ROOT=$_ejc_val
ejc_resolve_to_absolute "$OUTPUT_DIR_INPUT" "$REPO_ROOT";  OUTPUT_DIR=$_ejc_val

# ========================== Script-specific state ===========================

EJC_ELK_ROOT=$JAVA_TRACE_EXTERNAL_ELK_ROOT
TRACE_CLASSES_DIR=

# ========================== Cleanup =========================================

_trace_cleanup() {
  if [ -n "$TRACE_CLASSES_DIR" ] && [ -d "$TRACE_CLASSES_DIR" ]; then
    rm -rf "$TRACE_CLASSES_DIR"
  fi
}

ejc_register_cleanup _trace_cleanup

# ========================== Validation ======================================

ejc_validate_maven

if [ "$JAVA_TRACE_DRY_RUN" != "true" ]; then
  if ! command -v javac >/dev/null 2>&1; then
    echo "missing javac in PATH" >&2
    exit 1
  fi
  if ! command -v java >/dev/null 2>&1; then
    echo "missing java in PATH" >&2
    exit 1
  fi
fi

if [ ! -f "$JAVA_TRACE_BENCH_SOURCE" ]; then
  echo "missing java phase trace source: $JAVA_TRACE_BENCH_SOURCE" >&2
  exit 1
fi

# ========================== Clean check + Isolation =========================

ejc_check_clean_elk
ejc_create_isolation trace

# ========================== Patches + SNAPSHOT purge ========================

ejc_apply_patches
ejc_purge_snapshot_cache

# ========================== Build plugins ===================================

PREPARE_POM="$EJC_ELK_ROOT/build/pom.xml"
ejc_mvn_build_plugins "$PREPARE_POM"

# ========================== Collect classpath ===============================

echo "=== Collecting classpath ==="

CLASSPATH=""

select_latest_jar() {
  base_dir=$1
  pattern=$2
  find "$base_dir" -name "$pattern" 2>/dev/null \
    | grep -v sources \
    | grep -v javadoc \
    | sort -V \
    | tail -1
}

# Prepend preferred dependency versions first to avoid mixed-version classpath conflicts.
PREFERRED_GUAVA_JAR=$(select_latest_jar "${HOME}/.m2/repository/com/google/guava" "guava-*.jar")
PREFERRED_GUICE_JAR=$(select_latest_jar "${HOME}/.m2/repository/com/google/inject" "guice-*.jar")
PREFERRED_JAKARTA_INJECT_JAR=$(select_latest_jar "${HOME}/.m2/repository/jakarta/inject" "jakarta.inject-api-*.jar")

for jar in "$PREFERRED_GUAVA_JAR" "$PREFERRED_GUICE_JAR" "$PREFERRED_JAKARTA_INJECT_JAR"; do
  if [ -n "$jar" ] && [ -f "$jar" ]; then
    CLASSPATH="${CLASSPATH:+$CLASSPATH:}$jar"
  fi
done

# Add all target/classes directories from plugin modules
for d in "$EJC_ELK_ROOT"/plugins/*/target/classes; do
  if [ -d "$d" ]; then
    CLASSPATH="$CLASSPATH:$d"
  fi
done

# Add all target/classes directories from test modules (for PlainJavaInitialization)
for d in "$EJC_ELK_ROOT"/test/*/target/classes; do
  if [ -d "$d" ]; then
    CLASSPATH="$CLASSPATH:$d"
  fi
done

# Add dependency JARs from target/dependency/ directories (Tycho copies some here)
for jar in $(find "$EJC_ELK_ROOT" -name "*.jar" -path "*/target/dependency/*" 2>/dev/null); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Add Guava JARs from Maven local repo
if [ -n "$PREFERRED_GUAVA_JAR" ] && [ -f "$PREFERRED_GUAVA_JAR" ]; then
  CLASSPATH="$CLASSPATH:$PREFERRED_GUAVA_JAR"
fi

# Add Gson JARs from Maven local repo
for jar in $(find "${HOME}/.m2/repository/com/google/code/gson" -name "gson-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -2); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Add Eclipse Xtext/Xbase JARs (needed for ELK text format support).
# IMPORTANT: use select_latest_jar per module to pick only ONE version.
# Different xtext versions (e.g. 2.28.0 vs 2.36.0) have different signing
# certificates; mixing them on the classpath causes SecurityException
# ("signer information does not match").
for module in org.eclipse.xtext org.eclipse.xtext.util org.eclipse.xtext.common.types \
              org.eclipse.xtext.xbase org.eclipse.xtext.xbase.lib \
              org.eclipse.xtext.ecore org.eclipse.xtext.ide org.eclipse.xtext.smap; do
  jar=$(select_latest_jar "${HOME}/.m2/repository/org/eclipse/xtext/$module" "${module}-*.jar")
  if [ -n "$jar" ] && [ -f "$jar" ]; then
    CLASSPATH="$CLASSPATH:$jar"
  fi
done

# Add Tycho-resolved dependencies from target/
# Exclude xtext JARs — already added via select_latest_jar above (signer conflict guard).
for jar in $(find "$EJC_ELK_ROOT" -name "*.jar" -path "*/target/*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | grep -v xtext); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Add JUnit (PlainJavaInitialization may reference it at compile time)
for jar in $(find "${HOME}/.m2/repository/junit" -name "junit-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -1); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Add Google Inject (Guice) - needed by PlainJavaInitialization
if [ -n "$PREFERRED_GUICE_JAR" ] && [ -f "$PREFERRED_GUICE_JAR" ]; then
  CLASSPATH="$CLASSPATH:$PREFERRED_GUICE_JAR"
fi
# Add Jakarta Inject API (Guice 7+ runtime dependency)
if [ -n "$PREFERRED_JAKARTA_INJECT_JAR" ] && [ -f "$PREFERRED_JAKARTA_INJECT_JAR" ]; then
  CLASSPATH="$CLASSPATH:$PREFERRED_JAKARTA_INJECT_JAR"
fi
for jar in $(find "${HOME}/.m2/repository/javax/inject" -name "javax.inject-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -1); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Also try p2/osgi bundles that Tycho resolves into the target platform
# IMPORTANT: exclude xtext JARs — they are already added from Maven Central above
# (lines 166-168). p2/Tycho bundles are signed while Maven Central ones are not;
# mixing both causes SecurityException ("signer information does not match").
for jar in $(find "$EJC_ELK_ROOT" -name "*.jar" -path "*/.p2/*" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | grep -v xtext | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Tycho stores resolved p2 bundles in ~/.m2/repository/p2/osgi/bundle/
# Use -L to follow symlinks and glob to ensure traversal
for jar in $(find -L "${HOME}/.m2/repository/p2" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | grep -v xtext | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done
# Also add Tycho cache (Eclipse target platform JARs)
for jar in $(find -L "${HOME}/.m2/repository/.cache/tycho" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | grep -v xtext | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done
# Add org.osgi JARs from standard Maven repo
for jar in $(find -L "${HOME}/.m2/repository/org/osgi" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Broad search: find Gson JAR anywhere in .m2 if not found above
if ! echo "$CLASSPATH" | grep -qi "gson"; then
  for jar in $(find "${HOME}/.m2" -name "com.google.gson*.jar" -o -name "gson-*.jar" 2>/dev/null | grep -v sources | grep -v source | grep -v javadoc | head -3); do
    CLASSPATH="$CLASSPATH:$jar"
  done
fi

# Broad search: find Guava JAR anywhere in .m2 if not found above
if ! echo "$CLASSPATH" | grep -qi "guava"; then
  for jar in $(find "${HOME}/.m2" -name "com.google.guava*.jar" -o -name "guava-*.jar" 2>/dev/null | grep -v sources | grep -v source | grep -v javadoc | head -3); do
    CLASSPATH="$CLASSPATH:$jar"
  done
fi

# Strip leading colon
CLASSPATH=$(echo "$CLASSPATH" | sed 's/^://')

if [ -z "$CLASSPATH" ]; then
  echo "ERROR: no classpath entries found. Did the Maven build succeed?" >&2
  exit 1
fi

# ========================== Compile with javac ==============================

echo "=== Compiling ElkPhaseTraceExporter.java with javac ==="

TRACE_CLASSES_DIR=$(mktemp -d "${TMPDIR:-/tmp}/elk-trace-classes.XXXXXX")

if [ "$JAVA_TRACE_DRY_RUN" = "true" ]; then
  echo "ejc dry-run: javac -cp <classpath> -d $TRACE_CLASSES_DIR $JAVA_TRACE_BENCH_SOURCE"
else
  javac -cp "$CLASSPATH" -d "$TRACE_CLASSES_DIR" "$JAVA_TRACE_BENCH_SOURCE"
fi

# ========================== Run with java ===================================

echo "=== Running ElkPhaseTraceExporter ==="

FULL_CLASSPATH="$TRACE_CLASSES_DIR:$CLASSPATH"

set -- java -cp "$FULL_CLASSPATH" \
  "-Delk.trace.modelsRoot=$MODELS_ROOT" \
  "-Delk.trace.outputDir=$OUTPUT_DIR" \
  "-Delk.trace.limit=$JAVA_TRACE_LIMIT" \
  "-Delk.trace.include=$JAVA_TRACE_INCLUDE" \
  "-Delk.trace.exclude=$JAVA_TRACE_EXCLUDE" \
  "-Delk.trace.randomSeed=$JAVA_TRACE_RANDOM_SEED" \
  "-Delk.trace.prettyPrint=$JAVA_TRACE_PRETTY_PRINT" \
  org.eclipse.elk.graph.json.test.ElkPhaseTraceExporter

ejc_run_cmd "$@"

echo "java phase trace export finished: output=$OUTPUT_DIR"
