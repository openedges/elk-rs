#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

MODELS_ROOT_INPUT=${1:-external/elk-models}
OUTPUT_ROOT_INPUT=${2:-perf/model_parity}

MODEL_PARITY_CARGO_FLAGS=${MODEL_PARITY_CARGO_FLAGS:---release}

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

# Ensure elk-models submodule is initialized
if [ ! -f "$MODELS_ROOT/README.md" ] && [ -f "$REPO_ROOT/.gitmodules" ]; then
  echo "Initializing elk-models submodule..."
  git -C "$REPO_ROOT" submodule update --init external/elk-models
fi

MODEL_PARITY_PRETTY_PRINT=${MODEL_PARITY_PRETTY_PRINT:-false}
MODEL_PARITY_STOP_ON_ERROR=${MODEL_PARITY_STOP_ON_ERROR:-false}
MODEL_PARITY_ABS_TOL=${MODEL_PARITY_ABS_TOL:-1e-6}
MODEL_PARITY_MAX_DIFFS_PER_MODEL=${MODEL_PARITY_MAX_DIFFS_PER_MODEL:-20}
MODEL_PARITY_STRICT=${MODEL_PARITY_STRICT:-false}
MODEL_PARITY_RANDOM_SEED=${MODEL_PARITY_RANDOM_SEED:-1}
JAVA_PARITY_EXCLUDE_FILE=${JAVA_PARITY_EXCLUDE_FILE:-}

if [ -z "$JAVA_PARITY_EXCLUDE_FILE" ] && [ -f "$OUTPUT_ROOT/java_exclude.txt" ]; then
  JAVA_PARITY_EXCLUDE_FILE="$OUTPUT_ROOT/java_exclude.txt"
fi

JAVA_PARITY_EXCLUDE_FILE="$JAVA_PARITY_EXCLUDE_FILE" \
JAVA_PARITY_RANDOM_SEED="$MODEL_PARITY_RANDOM_SEED" \
  sh "$SCRIPT_DIR/run_java_model_parity_export.sh" "$MODELS_ROOT" "$JAVA_OUTPUT_DIR"

MODEL_PARITY_RANDOM_SEED="$MODEL_PARITY_RANDOM_SEED" \
  cargo run -p org-eclipse-elk-graph-json --bin model_parity_layout_runner \
  $MODEL_PARITY_CARGO_FLAGS \
  -- \
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

# Print report summary (match/drift/error/timeout counts)
if [ -f "$REPORT_FILE" ]; then
  echo ""
  echo "=== Report Summary ==="
  sed -n '/^|.*match\|^|.*Match\|^| Category/,/^$/p' "$REPORT_FILE" 2>/dev/null || true
  grep -E '(total models|matched|drift|error|timeout|compared)' "$REPORT_FILE" 2>/dev/null | head -10 || true
fi
