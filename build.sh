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

echo "==> copy static shell"
cp index.html dist/index.html

echo "==> done. dist/ contents:"
ls -lh dist
