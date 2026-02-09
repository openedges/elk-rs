#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

MODELS_ROOT_INPUT=${1:-external/elk-models}
OUTPUT_ROOT_INPUT=${2:-perf/model_parity}

case "$MODELS_ROOT_INPUT" in
  /*) MODELS_ROOT="$MODELS_ROOT_INPUT" ;;
  *) MODELS_ROOT="$REPO_ROOT/$MODELS_ROOT_INPUT" ;;
esac
case "$OUTPUT_ROOT_INPUT" in
  /*) OUTPUT_ROOT="$OUTPUT_ROOT_INPUT" ;;
  *) OUTPUT_ROOT="$REPO_ROOT/$OUTPUT_ROOT_INPUT" ;;
esac

JAVA_OUTPUT_DIR="$OUTPUT_ROOT/java"
JAVA_MANIFEST="$JAVA_OUTPUT_DIR/java_manifest.tsv"
RUST_LAYOUT_DIR="$OUTPUT_ROOT/rust/layout"
RUST_MANIFEST="$OUTPUT_ROOT/rust_manifest.tsv"
REPORT_FILE="$OUTPUT_ROOT/report.md"
DETAILS_FILE="$OUTPUT_ROOT/diff_details.tsv"

MODEL_PARITY_PRETTY_PRINT=${MODEL_PARITY_PRETTY_PRINT:-false}
MODEL_PARITY_STOP_ON_ERROR=${MODEL_PARITY_STOP_ON_ERROR:-false}
MODEL_PARITY_ABS_TOL=${MODEL_PARITY_ABS_TOL:-1e-6}
MODEL_PARITY_MAX_DIFFS_PER_MODEL=${MODEL_PARITY_MAX_DIFFS_PER_MODEL:-20}
MODEL_PARITY_STRICT=${MODEL_PARITY_STRICT:-false}
MODEL_PARITY_RANDOM_SEED=${MODEL_PARITY_RANDOM_SEED:-1}

JAVA_PARITY_RANDOM_SEED="$MODEL_PARITY_RANDOM_SEED" \
  sh "$SCRIPT_DIR/run_java_model_parity_export.sh" "$MODELS_ROOT" "$JAVA_OUTPUT_DIR"

MODEL_PARITY_RANDOM_SEED="$MODEL_PARITY_RANDOM_SEED" \
  cargo run -p org-eclipse-elk-graph-json --bin model_parity_layout_runner -- \
  --input-manifest "$JAVA_MANIFEST" \
  --output-manifest "$RUST_MANIFEST" \
  --rust-layout-dir "$RUST_LAYOUT_DIR" \
  --pretty-print "$MODEL_PARITY_PRETTY_PRINT" \
  --stop-on-error "$MODEL_PARITY_STOP_ON_ERROR"

STRICT_FLAG=
if [ "$MODEL_PARITY_STRICT" = "true" ]; then
  STRICT_FLAG=--strict
fi

python3 "$SCRIPT_DIR/compare_model_parity_layouts.py" \
  --manifest "$RUST_MANIFEST" \
  --report "$REPORT_FILE" \
  --details "$DETAILS_FILE" \
  --abs-tol "$MODEL_PARITY_ABS_TOL" \
  --max-diffs-per-model "$MODEL_PARITY_MAX_DIFFS_PER_MODEL" \
  $STRICT_FLAG

echo "model parity completed:"
echo "  report : $REPORT_FILE"
echo "  details: $DETAILS_FILE"
echo "  rust manifest: $RUST_MANIFEST"
