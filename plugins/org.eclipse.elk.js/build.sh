#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WASM_DIR="$SCRIPT_DIR/../org.eclipse.elk.wasm"
NAPI_DIR="$SCRIPT_DIR/../org.eclipse.elk.napi"
DIST_DIR="$SCRIPT_DIR/dist"

echo "=== elk-rs build ==="

# Clean
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/wasm"

# 1. WASM build
echo "--- Building WASM ---"
if command -v wasm-pack &> /dev/null; then
  (cd "$WASM_DIR" && wasm-pack build --target web --out-dir "$DIST_DIR/wasm" --release)
  # wasm-pack generates .gitignore (containing "*") and package.json in the output dir.
  # Remove them so npm pack can include the WASM files.
  rm -f "$DIST_DIR/wasm/.gitignore" "$DIST_DIR/wasm/package.json"
  echo "WASM build complete."
else
  echo "WARNING: wasm-pack not found. Skipping WASM build."
  echo "Install with: cargo install wasm-pack"
fi

# 2. Native addon build
echo "--- Building native addon ---"
if command -v npx &> /dev/null && [ -f "$NAPI_DIR/Cargo.toml" ]; then
  (cd "$NAPI_DIR" && npx napi build --release --platform --out-dir "$DIST_DIR")
  echo "Native addon build complete."
else
  echo "WARNING: npx/napi not found or napi crate missing. Skipping native build."
  echo "Install with: npm install -g @napi-rs/cli"
fi

echo "=== Build complete ==="
echo "Output: $DIST_DIR/"
ls -la "$DIST_DIR/"
