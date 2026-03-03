#!/bin/sh
# 5-Way Performance Benchmark Orchestration.
#
# Runs benchmarks sequentially across all available engines and collects
# results into a single directory for comparison.
#
# Usage:
#   sh scripts/run_perf_benchmark.sh [mode] [iterations] [warmup] [output_dir]
#
# Arguments:
#   mode        — synthetic (default) or models
#   iterations  — iterations per scenario (default: 20)
#   warmup      — warmup iterations (default: 3)
#   output_dir  — output directory (default: tests/perf)
#
# Environment variables:
#   PERF_ENGINES             — comma-separated engine list (default: rust_native,rust_api,elkjs,napi,wasm)
#   PERF_SKIP_JAVA           — skip Java benchmark (default: false)
#   PERF_SKIP_BUILD          — skip cargo/npm builds (default: false)
#   PERF_JS_ENGINES          — JS engines to benchmark (default: elkjs,napi,wasm)
#   PERF_MODEL_MANIFEST      — manifest TSV for models mode
#   PERF_MODEL_LIMIT         — max models (default: 50)
#   PERF_GENERATE_REPORT     — generate comparison report (default: true)
#
# Output:
#   $output_dir/rust_native.csv
#   $output_dir/rust_api.csv
#   $output_dir/java.csv           (if Java available)
#   $output_dir/elkjs.csv
#   $output_dir/napi.csv
#   $output_dir/wasm.csv
#   $output_dir/report.md          (comparison report)

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

MODE=${1:-synthetic}
ITERATIONS=${2:-20}
WARMUP=${3:-3}
OUTPUT_DIR=${4:-tests/perf}

PERF_SKIP_JAVA=${PERF_SKIP_JAVA:-false}
PERF_SKIP_BUILD=${PERF_SKIP_BUILD:-false}
PERF_JS_ENGINES=${PERF_JS_ENGINES:-elkjs,napi,wasm}
PERF_MODEL_MANIFEST=${PERF_MODEL_MANIFEST:-$REPO_ROOT/parity/model_parity/java/java_manifest.tsv}
PERF_MODEL_LIMIT=${PERF_MODEL_LIMIT:-50}
PERF_GENERATE_REPORT=${PERF_GENERATE_REPORT:-true}

# Resolve to absolute path
case "$OUTPUT_DIR" in
  /*) ;;
  *) OUTPUT_DIR="$(pwd)/$OUTPUT_DIR" ;;
esac

mkdir -p "$OUTPUT_DIR"

# -----------------------------------------------------------------------
# Preflight checks
# -----------------------------------------------------------------------
preflight_warn=""

if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  preflight_warn="${preflight_warn}  [warn] node not found — JS engines (elkjs/napi/wasm) will be skipped\n"
  PERF_JS_ENGINES=""
fi

JS_DIR="$REPO_ROOT/plugins/org.eclipse.elk.js"
if [ -n "$PERF_JS_ENGINES" ] && [ -d "$JS_DIR" ]; then
  # Check node_modules (elkjs dependency)
  if [ ! -d "$JS_DIR/node_modules/elkjs" ]; then
    preflight_warn="${preflight_warn}  [warn] elkjs not installed — run 'cd plugins/org.eclipse.elk.js && npm install' first\n"
    # Remove elkjs from engine list
    PERF_JS_ENGINES=$(echo "$PERF_JS_ENGINES" | sed 's/elkjs,\?//;s/,$//')
  fi
  # Check NAPI binary
  if echo "$PERF_JS_ENGINES" | grep -q "napi"; then
    if [ ! -f "$JS_DIR/dist/elk-rs.node" ] && ! ls "$JS_DIR"/dist/elk-rs.*.node >/dev/null 2>&1; then
      preflight_warn="${preflight_warn}  [warn] NAPI binary not found — run 'cd plugins/org.eclipse.elk.js && sh build.sh' or skip with PERF_JS_ENGINES=elkjs,wasm\n"
      PERF_JS_ENGINES=$(echo "$PERF_JS_ENGINES" | sed 's/napi,\?//;s/,$//')
    fi
  fi
  # Check WASM binary
  if echo "$PERF_JS_ENGINES" | grep -q "wasm"; then
    if [ ! -f "$JS_DIR/dist/wasm/org_eclipse_elk_wasm_bg.wasm" ]; then
      preflight_warn="${preflight_warn}  [warn] WASM binary not found — run 'cd plugins/org.eclipse.elk.js && sh build.sh' or skip with PERF_JS_ENGINES=elkjs,napi\n"
      PERF_JS_ENGINES=$(echo "$PERF_JS_ENGINES" | sed 's/wasm,\?//;s/,$//')
    fi
  fi
fi

if [ "$PERF_SKIP_JAVA" != "true" ]; then
  if ! command -v mvn >/dev/null 2>&1; then
    preflight_warn="${preflight_warn}  [warn] mvn not found — Java benchmark will be skipped\n"
    PERF_SKIP_JAVA=true
  elif [ ! -d "$REPO_ROOT/external/elk" ]; then
    preflight_warn="${preflight_warn}  [warn] external/elk not found — Java benchmark will be skipped (run 'git submodule update --init')\n"
    PERF_SKIP_JAVA=true
  fi
fi

echo "======================================"
echo "5-Way Performance Benchmark"
echo "======================================"
echo "  Mode:       $MODE"
echo "  Iterations: $ITERATIONS"
echo "  Warmup:     $WARMUP"
echo "  Output:     $OUTPUT_DIR"
if [ -n "$preflight_warn" ]; then
  echo ""
  printf "%b" "$preflight_warn"
fi
echo ""

errors=0

# -----------------------------------------------------------------------
# Build unified Rust benchmark binary
# -----------------------------------------------------------------------
if [ "$PERF_SKIP_BUILD" != "true" ]; then
  cargo build -p org-eclipse-elk-graph-json --release --bin perf_benchmark --features org-eclipse-elk-graph-json/mimalloc-alloc 2>&1 | tail -1
fi

# -----------------------------------------------------------------------
# 1. Rust native benchmark (direct ElkNode, no JSON overhead)
# -----------------------------------------------------------------------
if [ "$MODE" = "synthetic" ]; then
  echo "--- [1/5] Rust native (direct ElkNode) ---"
  if cargo run -p org-eclipse-elk-graph-json --release --bin perf_benchmark --features org-eclipse-elk-graph-json/mimalloc-alloc -- \
    --engine rust_native --mode synthetic \
    --iterations "$ITERATIONS" --warmup "$WARMUP" --output "$OUTPUT_DIR/rust_native.csv"; then
    echo "  -> $OUTPUT_DIR/rust_native.csv"
  else
    echo "  -> FAILED" >&2
    errors=$((errors + 1))
  fi
  echo ""
else
  echo "--- [1/5] Rust native: skipped (models mode uses JSON API only) ---"
  echo ""
fi

# -----------------------------------------------------------------------
# 2. Rust API benchmark (layout_json, same path as NAPI/WASM)
# -----------------------------------------------------------------------
echo "--- [2/5] Rust API (layout_json) ---"

RUST_API_ARGS="--engine rust_api --mode $MODE --iterations $ITERATIONS --warmup $WARMUP --output $OUTPUT_DIR/rust_api.csv"
if [ "$MODE" = "models" ]; then
  RUST_API_ARGS="$RUST_API_ARGS --manifest $PERF_MODEL_MANIFEST --limit $PERF_MODEL_LIMIT"
fi

# shellcheck disable=SC2086
if cargo run -p org-eclipse-elk-graph-json --release --bin perf_benchmark --features org-eclipse-elk-graph-json/mimalloc-alloc -- $RUST_API_ARGS; then
  echo "  -> $OUTPUT_DIR/rust_api.csv"
else
  echo "  -> FAILED" >&2
  errors=$((errors + 1))
fi
echo ""

# -----------------------------------------------------------------------
# 3. Java benchmark
# -----------------------------------------------------------------------
if [ "$PERF_SKIP_JAVA" = "true" ]; then
  echo "--- [3/5] Java: skipped (PERF_SKIP_JAVA=true) ---"
else
  echo "--- [3/5] Java (RecursiveGraphLayoutEngine) ---"
  java_ok=false
  if [ "$MODE" = "synthetic" ]; then
    # Use existing layered issue scenario runner for synthetic mode
    if JAVA_PARITY_BUILD_PLUGINS=false sh "$SCRIPT_DIR/run_java_parity_layered_issue_scenarios.sh" \
        "layered_small,layered_medium,layered_large,layered_xlarge,force_medium,stress_medium,mrtree_medium,radial_medium,rectpacking_medium,routing_polyline,routing_orthogonal,routing_splines,crossmin_layer_sweep,crossmin_none,hierarchy_flat,hierarchy_nested" \
        "$ITERATIONS" "$WARMUP" "$OUTPUT_DIR/java.csv"; then
      java_ok=true
    fi
  else
    if sh "$SCRIPT_DIR/run_java_model_benchmark.sh" "$MODE" "$ITERATIONS" "$WARMUP" "$OUTPUT_DIR/java.csv"; then
      java_ok=true
    fi
  fi
  if [ "$java_ok" = "true" ]; then
    # Convert legacy Java CSV (no engine column) to standard format if needed
    if [ -f "$OUTPUT_DIR/java.csv" ] && ! head -1 "$OUTPUT_DIR/java.csv" | grep -q "engine"; then
      tmp="$OUTPUT_DIR/java.csv.tmp"
      echo "timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec" > "$tmp"
      while IFS= read -r line; do
        [ -z "$line" ] && continue
        ts=$(echo "$line" | cut -d, -f1)
        rest=$(echo "$line" | cut -d, -f2-)
        echo "${ts},java,${rest}" >> "$tmp"
      done < "$OUTPUT_DIR/java.csv"
      mv "$tmp" "$OUTPUT_DIR/java.csv"
    fi
    echo "  -> $OUTPUT_DIR/java.csv"
  else
    echo "  -> FAILED (Java benchmark requires Maven + external/elk)" >&2
    errors=$((errors + 1))
  fi
fi
echo ""

# -----------------------------------------------------------------------
# 4-5. JS engines (elkjs, NAPI, WASM)
# -----------------------------------------------------------------------
echo "--- [4-5/5] JS engines ($PERF_JS_ENGINES) ---"
JS_DIR="$REPO_ROOT/plugins/org.eclipse.elk.js"

if [ ! -d "$JS_DIR" ]; then
  echo "  -> SKIPPED (JS directory not found)" >&2
else
  JS_BENCH_ARGS="--mode $MODE --engines $PERF_JS_ENGINES --iterations $ITERATIONS --warmup $WARMUP"
  if [ "$MODE" = "models" ]; then
    JS_BENCH_ARGS="$JS_BENCH_ARGS --manifest $PERF_MODEL_MANIFEST --limit $PERF_MODEL_LIMIT"
  fi

  # Run each JS engine separately for individual CSV output
  OLD_IFS=$IFS
  IFS=','
  # shellcheck disable=SC2086
  set -- $PERF_JS_ENGINES
  IFS=$OLD_IFS

  for engine in "$@"; do
    trimmed=$(printf '%s' "$engine" | awk '{ gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print }')
    [ -z "$trimmed" ] && continue
    echo "  [$trimmed]"
    if node "$JS_DIR/bench/bench.mjs" --mode "$MODE" --engines "$trimmed" \
      --iterations "$ITERATIONS" --warmup "$WARMUP" \
      --output "$OUTPUT_DIR/$trimmed.csv" 2>&1 | grep -v "^$"; then
      echo "    -> $OUTPUT_DIR/$trimmed.csv"
    else
      echo "    -> FAILED" >&2
      errors=$((errors + 1))
    fi
  done
fi
echo ""

# -----------------------------------------------------------------------
# Generate comparison report
# -----------------------------------------------------------------------
if [ "$PERF_GENERATE_REPORT" = "true" ]; then
  echo "--- Generating comparison report ---"
  if command -v python3 >/dev/null 2>&1; then
    if python3 "$SCRIPT_DIR/compare_perf_results.py" "$OUTPUT_DIR" "$OUTPUT_DIR/report.md"; then
      echo "  -> $OUTPUT_DIR/report.md"
    else
      echo "  -> Report generation failed" >&2
      errors=$((errors + 1))
    fi
  else
    echo "  -> SKIPPED (python3 not available)" >&2
  fi
fi

echo ""
echo "======================================"
echo "Benchmark complete. Errors: $errors"
echo "Results: $OUTPUT_DIR/"
ls -la "$OUTPUT_DIR/"*.csv 2>/dev/null || true
echo "======================================"

exit $errors
