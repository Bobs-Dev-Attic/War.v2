#!/usr/bin/env bash
set -euo pipefail
cargo build --release --target wasm32-unknown-unknown
rm -rf dist
mkdir -p dist
wasm-bindgen --target web --out-dir dist --out-name war_v2 target/wasm32-unknown-unknown/release/war_v2.wasm
cp index.html dist/index.html
if [ -d assets ]; then cp -R assets dist/assets; fi
