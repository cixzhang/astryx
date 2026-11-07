#!/usr/bin/env bash
# Builds the web demo into crates/astryx-web/dist (or $OUT).
#
#   scripts/build.sh            web-release profile, then wasm-opt
#   BASELINE=1 scripts/build.sh workspace release profile, no wasm-opt
#
# Needs: cargo with the wasm32-unknown-unknown target, wasm-bindgen-cli at the
# same version as the wasm-bindgen crate and Python fontTools for the font
# subsets. Optional: wasm-opt (binaryen).
set -euo pipefail

crate="$(cd "$(dirname "$0")/.." && pwd)"
workspace="$(cd "$crate/../.." && pwd)"
out="${OUT:-$crate/dist}"
target_dir="${CARGO_TARGET_DIR:-$workspace/target}"

# web-release (root Cargo.toml): opt-level s, fat LTO, panic=abort, symbols kept
# for wasm-bindgen; wasm-opt strips them afterward.
profile=web-release
if [ "${BASELINE:-0}" = 1 ]; then
  profile=release
fi

cargo build --manifest-path "$workspace/Cargo.toml" -p astryx-web --example demo \
  --target wasm32-unknown-unknown --profile "$profile"

rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" \
  "$target_dir/wasm32-unknown-unknown/$profile/examples/demo.wasm"

if [ "${BASELINE:-0}" != 1 ] && command -v wasm-opt >/dev/null; then
  # The features Rust enables by default for wasm32 must be allowed explicitly.
  wasm-opt -Oz --strip-debug --strip-producers \
    --enable-bulk-memory --enable-mutable-globals --enable-sign-ext \
    --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue \
    "$out/demo_bg.wasm" -o "$out/demo_bg.wasm.opt"
  mv "$out/demo_bg.wasm.opt" "$out/demo_bg.wasm"
fi

cp "$crate/web/index.html" "$out/index.html"
"${PYTHON:-python3}" "$crate/scripts/subset-fonts.py" "$out/fonts"

echo "built $out"
