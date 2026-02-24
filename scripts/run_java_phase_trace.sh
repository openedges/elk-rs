#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

MODELS_ROOT_INPUT=${1:-external/elk-models}
OUTPUT_DIR_INPUT=${2:-perf/model_parity/java_trace}

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

JAVA_TRACE_BENCH_SOURCE=${JAVA_TRACE_BENCH_SOURCE:-$SCRIPT_DIR/java/ElkPhaseTraceExporter.java}

JAVA_TRACE_LIMIT=${JAVA_TRACE_LIMIT:-0}
JAVA_TRACE_INCLUDE=${JAVA_TRACE_INCLUDE:-}
JAVA_TRACE_EXCLUDE=${JAVA_TRACE_EXCLUDE:-}
JAVA_TRACE_RANDOM_SEED=${JAVA_TRACE_RANDOM_SEED:-1}
JAVA_TRACE_PRETTY_PRINT=${JAVA_TRACE_PRETTY_PRINT:-true}

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

ELK_ROOT=$JAVA_TRACE_EXTERNAL_ELK_ROOT
isolation_mode=none
isolated_worktree_dir=
isolated_copy_dir=
TRACE_CLASSES_DIR=

cleanup() {
  if [ -n "$TRACE_CLASSES_DIR" ] && [ -d "$TRACE_CLASSES_DIR" ]; then
    rm -rf "$TRACE_CLASSES_DIR"
  fi

  if [ "$isolation_mode" = "worktree" ] && [ -n "$isolated_worktree_dir" ]; then
    git -C "$JAVA_TRACE_EXTERNAL_ELK_ROOT" worktree remove --force "$isolated_worktree_dir" >/dev/null 2>&1 || true
  fi
  if [ -n "$isolated_worktree_dir" ]; then
    rm -rf "$isolated_worktree_dir"
  fi
  if [ -n "$isolated_copy_dir" ]; then
    rm -rf "$isolated_copy_dir"
  fi
}

trap cleanup EXIT INT TERM

# ========================== Validation ==========================

if [ "$JAVA_TRACE_DRY_RUN" != "true" ]; then
  case "$JAVA_TRACE_MVN_BIN" in
    */*)
      if [ ! -x "$JAVA_TRACE_MVN_BIN" ]; then
        echo "maven command is not executable: $JAVA_TRACE_MVN_BIN" >&2
        exit 1
      fi
      ;;
    *)
      if ! command -v "$JAVA_TRACE_MVN_BIN" >/dev/null 2>&1; then
        echo "missing maven command in PATH: $JAVA_TRACE_MVN_BIN" >&2
        exit 1
      fi
      ;;
  esac

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

# ========================== Clean ELK check ==========================

if [ "$JAVA_TRACE_DRY_RUN" != "true" ] && [ "$JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK" = "true" ]; then
  if ! git -C "$JAVA_TRACE_EXTERNAL_ELK_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK=true but external ELK root is not a git worktree: $JAVA_TRACE_EXTERNAL_ELK_ROOT" >&2
    echo "set JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK=false to bypass this guard." >&2
    exit 1
  fi

  dirty_status=$(git -C "$JAVA_TRACE_EXTERNAL_ELK_ROOT" status --porcelain 2>/dev/null || true)
  if [ -n "$dirty_status" ]; then
    echo "external ELK tree has local changes; refusing phase trace export to protect external/elk state." >&2
    printf "%s\n" "$dirty_status" | sed -n '1,20p' >&2
    dirty_lines=$(printf "%s\n" "$dirty_status" | wc -l | awk '{print $1}')
    if [ "$dirty_lines" -gt 20 ]; then
      echo "... (showing first 20 of $dirty_lines changed paths)" >&2
    fi
    echo "set JAVA_TRACE_REQUIRE_CLEAN_EXTERNAL_ELK=false to bypass this guard." >&2
    exit 1
  fi
fi

# ========================== Isolation (worktree/copy) ==========================

if [ "$JAVA_TRACE_DRY_RUN" != "true" ] && [ "$JAVA_TRACE_EXTERNAL_ISOLATE" = "true" ]; then
  isolated_worktree_dir=$(mktemp -d "$JAVA_TRACE_EXTERNAL_WORKTREE_ROOT/elk-java-trace-worktree.XXXXXX")
  if [ -d "$isolated_worktree_dir" ]; then
    rmdir "$isolated_worktree_dir"
  fi
  if git -C "$JAVA_TRACE_EXTERNAL_ELK_ROOT" worktree add --detach "$isolated_worktree_dir" HEAD >/dev/null 2>&1; then
    ELK_ROOT=$isolated_worktree_dir
    isolation_mode=worktree
  else
    rm -rf "$isolated_worktree_dir"
    isolated_worktree_dir=
    isolated_copy_dir=$(mktemp -d "$JAVA_TRACE_EXTERNAL_WORKTREE_ROOT/elk-java-trace-copy.XXXXXX")
    cp -R "$JAVA_TRACE_EXTERNAL_ELK_ROOT"/. "$isolated_copy_dir"/
    ELK_ROOT=$isolated_copy_dir
    isolation_mode=copy
    echo "warning: failed to create git worktree; using copied external/elk tree at: $isolated_copy_dir" >&2
  fi
fi

# ========================== Helper ==========================

run_cmd() {
  if [ "$JAVA_TRACE_DRY_RUN" = "true" ]; then
    printf "java trace dry-run:"
    for arg in "$@"; do
      printf " %s" "$arg"
    done
    printf "\n"
    return
  fi

  "$@"
}

# ========================== Step 1: Build ELK plugins with Maven ==========================

PREPARE_POM="$ELK_ROOT/build/pom.xml"

if [ "$JAVA_TRACE_BUILD_PLUGINS" = "true" ]; then
  echo "=== Building ELK plugins with Maven ==="
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
  run_cmd "$@"
fi

# ========================== Step 2: Collect classpath ==========================

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
for d in "$ELK_ROOT"/plugins/*/target/classes; do
  if [ -d "$d" ]; then
    CLASSPATH="$CLASSPATH:$d"
  fi
done

# Add all target/classes directories from test modules (for PlainJavaInitialization)
for d in "$ELK_ROOT"/test/*/target/classes; do
  if [ -d "$d" ]; then
    CLASSPATH="$CLASSPATH:$d"
  fi
done

# Add dependency JARs from target/dependency/ directories (Tycho copies some here)
for jar in $(find "$ELK_ROOT" -name "*.jar" -path "*/target/dependency/*" 2>/dev/null); do
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

# Add Eclipse Xtext/Xbase JARs (needed for some ELK text format support)
for jar in $(find "${HOME}/.m2/repository/org/eclipse/xtext" -name "*.jar" 2>/dev/null | grep -v sources | grep -v javadoc | head -20); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Add Tycho-resolved dependencies from target/
for jar in $(find "$ELK_ROOT" -name "*.jar" -path "*/target/*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc); do
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
for jar in $(find "$ELK_ROOT" -name "*.jar" -path "*/.p2/*" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done

# Tycho stores resolved p2 bundles in ~/.m2/repository/p2/osgi/bundle/
# Use -L to follow symlinks and glob to ensure traversal
for jar in $(find -L "${HOME}/.m2/repository/p2" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | head -200); do
  CLASSPATH="$CLASSPATH:$jar"
done
# Also add Tycho cache (Eclipse target platform JARs)
for jar in $(find -L "${HOME}/.m2/repository/.cache/tycho" -name "*.jar" 2>/dev/null | grep -v '\.source' | grep -v sources | grep -v javadoc | head -200); do
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

# ========================== Step 3: Compile with javac ==========================

echo "=== Compiling ElkPhaseTraceExporter.java with javac ==="

TRACE_CLASSES_DIR=$(mktemp -d "${TMPDIR:-/tmp}/elk-trace-classes.XXXXXX")

if [ "$JAVA_TRACE_DRY_RUN" = "true" ]; then
  echo "java trace dry-run: javac -cp <classpath> -d $TRACE_CLASSES_DIR $JAVA_TRACE_BENCH_SOURCE"
else
  javac -cp "$CLASSPATH" -d "$TRACE_CLASSES_DIR" "$JAVA_TRACE_BENCH_SOURCE"
fi

# ========================== Step 4: Run with java ==========================

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

run_cmd "$@"

echo "java phase trace export finished: output=$OUTPUT_DIR"
