#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

# Required: path to a single model file
MODEL_PATH=${1:-}
RANDOM_SEED=${2:-1}
OUTPUT_FILE=${3:-/dev/stderr}

if [ -z "$MODEL_PATH" ]; then
  echo "Usage: $0 <model-path> [randomSeed] [output-file]" >&2
  echo "  model-path:  path to .elkt/.elkg/.json model file" >&2
  echo "  randomSeed:  random seed (default: 1)" >&2
  echo "  output-file: trace output file (default: stderr)" >&2
  exit 1
fi

# Resolve relative model path
case "$MODEL_PATH" in
  /*) ;;
  *) MODEL_PATH="$REPO_ROOT/$MODEL_PATH" ;;
esac

if [ ! -f "$MODEL_PATH" ]; then
  echo "Model file not found: $MODEL_PATH" >&2
  exit 1
fi

JAVA_TRACE_MVN_BIN=${JAVA_TRACE_MVN_BIN:-mvn}
JAVA_TRACE_BUILD_PLUGINS=${JAVA_TRACE_BUILD_PLUGINS:-true}
JAVA_TRACE_EXTERNAL_ELK_ROOT=${JAVA_TRACE_EXTERNAL_ELK_ROOT:-$REPO_ROOT/external/elk}
JAVA_TRACE_EXTERNAL_ISOLATE=${JAVA_TRACE_EXTERNAL_ISOLATE:-false}
JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK=${JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK:-false}
JAVA_TRACE_EXTERNAL_WORKTREE_ROOT=${JAVA_TRACE_EXTERNAL_WORKTREE_ROOT:-${TMPDIR:-/tmp}}
JAVA_TRACE_PREPARE_ARGS=${JAVA_TRACE_PREPARE_ARGS:--DskipTests -DskipITs}
JAVA_TRACE_MVN_LOCAL_REPO=${JAVA_TRACE_MVN_LOCAL_REPO:-}
JAVA_TRACE_MVN_ARGS=${JAVA_TRACE_MVN_ARGS:-}

ELK_ROOT=$JAVA_TRACE_EXTERNAL_ELK_ROOT
TRACE_CLASSES_DIR=

cleanup() {
  if [ -n "$TRACE_CLASSES_DIR" ] && [ -d "$TRACE_CLASSES_DIR" ]; then
    rm -rf "$TRACE_CLASSES_DIR"
  fi
}

trap cleanup EXIT INT TERM

# ========================== Step 1: Build ELK plugins ==========================

PREPARE_POM="$ELK_ROOT/build/pom.xml"

if [ "$JAVA_TRACE_BUILD_PLUGINS" = "true" ]; then
  echo "=== Building ELK plugins with Maven ===" >&2
  set -- "$JAVA_TRACE_MVN_BIN" -f "$PREPARE_POM"
  if [ -n "$JAVA_TRACE_MVN_LOCAL_REPO" ]; then
    set -- "$@" "-Dmaven.repo.local=$JAVA_TRACE_MVN_LOCAL_REPO"
  fi
  if [ -n "$JAVA_TRACE_PREPARE_ARGS" ]; then
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_TRACE_PREPARE_ARGS
  fi
  if [ -n "$JAVA_TRACE_MVN_ARGS" ]; then
    # shellcheck disable=SC2086
    set -- "$@" $JAVA_TRACE_MVN_ARGS
  fi
  set -- "$@" install
  "$@" >&2
fi

# ========================== Step 2: Collect classpath ==========================

echo "=== Collecting classpath ===" >&2

CLASSPATH=""

for d in "$ELK_ROOT"/plugins/*/target/classes; do
  [ -d "$d" ] && CLASSPATH="$CLASSPATH:$d"
done
for d in "$ELK_ROOT"/test/*/target/classes; do
  [ -d "$d" ] && CLASSPATH="$CLASSPATH:$d"
done
for jar in $(find "$ELK_ROOT" -name "*.jar" -path "*/target/dependency/*" 2>/dev/null); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find "${HOME}/.m2/repository/com/google/guava" -name "guava-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -5); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find "${HOME}/.m2/repository/com/google/code/gson" -name "gson-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -2); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find "${HOME}/.m2/repository/org/eclipse/emf" -name "*.jar" 2>/dev/null | grep -v sources | grep -v javadoc); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find "$ELK_ROOT" -name "*.jar" -path "*/target/*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find "${HOME}/.m2/repository/junit" -name "junit-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -1); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find "${HOME}/.m2/repository/com/google/inject" -name "guice-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -3); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find "${HOME}/.m2/repository/javax/inject" -name "javax.inject-*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -1); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find -L "${HOME}/.m2/repository/p2" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find -L "${HOME}/.m2/repository/.cache/tycho" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done
for jar in $(find -L "${HOME}/.m2/repository/org/osgi" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | head -20); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Broad fallback searches
if ! echo "$CLASSPATH" | grep -qi "gson"; then
  for jar in $(find "${HOME}/.m2" -name "com.google.gson*.jar" -o -name "gson-*.jar" 2>/dev/null | grep -v sources | grep -v source | grep -v javadoc | head -3); do
    CLASSPATH="$CLASSPATH:$jar"
  done
fi
if ! echo "$CLASSPATH" | grep -qi "guava"; then
  for jar in $(find "${HOME}/.m2" -name "com.google.guava*.jar" -o -name "guava-*.jar" 2>/dev/null | grep -v sources | grep -v source | grep -v javadoc | head -3); do
    CLASSPATH="$CLASSPATH:$jar"
  done
fi
if ! echo "$CLASSPATH" | grep -qi "emf"; then
  for jar in $(find "${HOME}/.m2" -name "org.eclipse.emf*.jar" 2>/dev/null | grep -v sources | grep -v source | grep -v javadoc | head -20); do
    CLASSPATH="$CLASSPATH:$jar"
  done
fi

CLASSPATH=$(echo "$CLASSPATH" | sed 's/^://')

if [ -z "$CLASSPATH" ]; then
  echo "ERROR: no classpath entries found. Did the Maven build succeed?" >&2
  exit 1
fi

# ========================== Step 3: Compile ==========================

echo "=== Compiling TracingRandom.java + ElkRandomTraceRunner.java ===" >&2

TRACE_CLASSES_DIR=$(mktemp -d "${TMPDIR:-/tmp}/elk-random-trace-classes.XXXXXX")

javac -cp "$CLASSPATH" -d "$TRACE_CLASSES_DIR" \
  "$SCRIPT_DIR/java/TracingRandom.java" \
  "$SCRIPT_DIR/java/ElkRandomTraceRunner.java"

# ========================== Step 4: Run ==========================

echo "=== Running ElkRandomTraceRunner ===" >&2

FULL_CLASSPATH="$TRACE_CLASSES_DIR:$CLASSPATH"

if [ "$OUTPUT_FILE" = "/dev/stderr" ]; then
  java -cp "$FULL_CLASSPATH" \
    org.eclipse.elk.graph.json.test.ElkRandomTraceRunner \
    "$MODEL_PATH" "$RANDOM_SEED"
else
  java -cp "$FULL_CLASSPATH" \
    org.eclipse.elk.graph.json.test.ElkRandomTraceRunner \
    "$MODEL_PATH" "$RANDOM_SEED" 2>"$OUTPUT_FILE"
  echo "Java random trace written to: $OUTPUT_FILE" >&2
fi
