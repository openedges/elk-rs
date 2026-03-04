#!/bin/sh
# Run full phase-step trace parity: Java trace + manifest + Rust trace + comparison.
#
# Usage:
#   sh scripts/run_full_trace_parity.sh [models_root] [output_base]
#
# Defaults:
#   models_root  = external/elk-models
#   output_base  = tests/model_parity
#
# Prerequisites:
#   - Java trace already generated (or will be generated in step 1)
#   - Rust built in release mode
#   - Python 3.8+ available

set -e

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

MODELS_ROOT="${1:-external/elk-models}"
OUTPUT_BASE="${2:-tests/model_parity}"

JAVA_TRACE_DIR="${OUTPUT_BASE}/java_trace"
RUST_TRACE_DIR="${OUTPUT_BASE}/rust_trace"
JAVA_OUTPUT_DIR="${OUTPUT_BASE}/../model_parity_full/java"
MANIFEST="${OUTPUT_BASE}/manifest.tsv"
RUST_LAYOUT_DIR="/tmp/elk-rs-full-trace-layouts"

echo "=== Full Trace Parity ==="
echo "Models root:   ${MODELS_ROOT}"
echo "Output base:   ${OUTPUT_BASE}"
echo ""

# ---------------------------------------------------------------
# Step 1: Java trace (skip if SKIP_JAVA_TRACE is set)
# ---------------------------------------------------------------
if [ "${SKIP_JAVA_TRACE:-}" = "true" ]; then
    echo "[1/4] Skipping Java trace generation (SKIP_JAVA_TRACE=true)"
else
    echo "[1/4] Generating Java trace..."
    sh scripts/java_model_phase_step_trace.sh "${MODELS_ROOT}" "${JAVA_TRACE_DIR}"
fi

# ---------------------------------------------------------------
# Step 2: Build unified manifest (Java rows + .json models)
# ---------------------------------------------------------------
echo ""
echo "[2/4] Building manifest..."
python3 "${SCRIPT_DIR}/generate_full_trace_manifest.py" \
    --models-root "${MODELS_ROOT}" \
    --java-output-dir "${JAVA_OUTPUT_DIR}" \
    --output "${MANIFEST}"

# ---------------------------------------------------------------
# Step 3: Rust trace
# ---------------------------------------------------------------
echo ""
echo "[3/4] Running Rust trace..."
mkdir -p "${RUST_LAYOUT_DIR}"
cargo run -p org-eclipse-elk-graph-json --bin model_parity_layout_runner --release -- \
    --input-manifest "${MANIFEST}" \
    --output-manifest /dev/null \
    --rust-layout-dir "${RUST_LAYOUT_DIR}" \
    --trace-dir "${RUST_TRACE_DIR}"

# ---------------------------------------------------------------
# Step 4: Compare traces (report missing models)
# ---------------------------------------------------------------
echo ""
echo "[4/4] Comparing traces..."
python3 scripts/compare_phase_traces.py \
    "${JAVA_TRACE_DIR}" "${RUST_TRACE_DIR}" --batch --report-missing

echo ""
echo "=== Full Trace Parity Complete ==="
