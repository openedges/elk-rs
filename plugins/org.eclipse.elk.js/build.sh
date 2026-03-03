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
  (cd "$NAPI_DIR" && npx napi build --release --platform --output-dir "$DIST_DIR")
  # Copy to platform package directory for local dev/publish
  PLATFORM_DIR="$SCRIPT_DIR/npm"
  if [ -d "$PLATFORM_DIR" ]; then
    for f in "$DIST_DIR"/elk-rs.*.node; do
      [ -e "$f" ] || continue
      BASENAME=$(basename "$f" .node)
      TRIPLE=$(echo "$BASENAME" | sed 's/^elk-rs\.//')
      if [ -d "$PLATFORM_DIR/$TRIPLE" ]; then
        cp "$f" "$PLATFORM_DIR/$TRIPLE/"
        echo "Copied $BASENAME.node to npm/$TRIPLE/"
      fi
    done
  fi
  echo "Native addon build complete."
else
  echo "WARNING: npx/napi not found or napi crate missing. Skipping native build."
  echo "Install with: npm install -g @napi-rs/cli"
fi

echo "=== Build complete ==="
echo "Output: $DIST_DIR/"
ls -la "$DIST_DIR/"
