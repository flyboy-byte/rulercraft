#!/bin/sh
# Builds the RulerCraft SolidCraft plugin package (docs/specs/solidcraft-plugin.md).
# Needs: rustup target add wasm32-unknown-unknown; cargo install wasm-tools.
# Optional: solidcraft-cli on PATH (or SOLIDCRAFT_CLI) to check and test the package.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
cargo build --release --target wasm32-unknown-unknown
wasm-tools component new target/wasm32-unknown-unknown/release/rulercraft_solidcraft_plugin.wasm \
  -o package/rulercraft.wasm
# SolidCraft links no WASI and no network: refuse a component that imports either.
if wasm-tools component wit package/rulercraft.wasm | grep -Ei 'wasi|http|socket'; then
  echo "the component imports WASI or network interfaces; SolidCraft would refuse it" >&2
  exit 1
fi
cp ../../fixtures/known/hole-block.stl ../../fixtures/known/stepped-cylinder.stl ../../fixtures/known/bracket.stl ../../fixtures/known/l-plate.stl ../../fixtures/known/round-plate.stl ../../fixtures/known/counterbore.stl ../../fixtures/known/window.stl package/tests/
mkdir -p dist
rm -f dist/rulercraft-solidcraft-plugin.zip
(cd package && zip -q ../dist/rulercraft-solidcraft-plugin.zip plugin.json rulercraft.wasm)
echo "built dist/rulercraft-solidcraft-plugin.zip"
cli=${SOLIDCRAFT_CLI:-solidcraft-cli}
if command -v "$cli" >/dev/null 2>&1; then
  "$cli" plugin check package
  "$cli" plugin test package
else
  echo "solidcraft-cli not found: skipped plugin check and plugin test"
fi
