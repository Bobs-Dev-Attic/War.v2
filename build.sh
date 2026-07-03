#!/usr/bin/env bash
#
# Build the Bevy app to WebAssembly and assemble the static site in ./dist.
#
# Requires:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version =0.2.126   # must match Cargo.lock
#
# The committed ./dist is exactly what Vercel serves (see vercel.json).

set -euo pipefail

CRATE=war_v2
TARGET_DIR="target/wasm32-unknown-unknown/release"

echo "==> cargo build (wasm32, release)"
cargo build --release --target wasm32-unknown-unknown

echo "==> wasm-bindgen (--target web)"
rm -rf dist
mkdir -p dist
wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir dist \
  --out-name "$CRATE" \
  "$TARGET_DIR/$CRATE.wasm"

if command -v wasm-opt >/dev/null 2>&1; then
  echo "==> wasm-opt (-Os) shrinking $CRATE""_bg.wasm"
  # -all enables the post-MVP wasm features (bulk-memory, sign-ext, …) that
  # rustc emits; without them wasm-opt rejects the module during validation.
  wasm-opt -Os -all --strip-debug "dist/${CRATE}_bg.wasm" -o "dist/${CRATE}_bg.wasm"
else
  echo "==> wasm-opt not found; skipping size optimization"
fi

echo "==> copy static shell"
cp index.html dist/index.html

echo "==> done. dist/ contents:"
ls -lh dist
