#!/usr/bin/env bash
# Builds the spike infra gallery for the web into dist/ (or $OUT): wasm, the page,
# and the font subsets. Needs the wasm32 target, wasm-bindgen-cli 0.2.130 and
# Python fontTools; wasm-opt is used when present.
set -euo pipefail
crate="$(cd "$(dirname "$0")/.." && pwd)"
workspace="$(cd "$crate/../.." && pwd)"
web_crate="$(cd "$workspace/../../rust/crates/astryx-web" && pwd)"
out="${OUT:-$crate/dist}"
target_dir="${CARGO_TARGET_DIR:-$workspace/target}"
profile="${PROFILE:-web-release}"
python="${PYTHON:-python3}"
rm -rf "$out"; mkdir -p "$out"
cargo build --manifest-path "$workspace/Cargo.toml" -p astryx-spike-infra-gallery-web \
  --lib --target wasm32-unknown-unknown --profile "$profile"
wasm-bindgen --target web --no-typescript --out-dir "$out" \
  "$target_dir/wasm32-unknown-unknown/$profile/astryx_spike_infra_gallery_web.wasm"
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz --strip-debug --strip-producers \
    --enable-bulk-memory --enable-mutable-globals --enable-sign-ext \
    --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue \
    "$out/astryx_spike_infra_gallery_web_bg.wasm" -o "$out/opt.wasm"
  mv "$out/opt.wasm" "$out/astryx_spike_infra_gallery_web_bg.wasm"
fi
cp "$crate/web/infra.html" "$out/"
"$python" "$web_crate/scripts/subset-fonts.py" "$out/fonts"
echo "built $out"
