#!/usr/bin/env bash
# Builds the web demo and the gallery into crates/astryx-web/dist (or $OUT).
#
#   scripts/build.sh            web-release profile, then wasm-opt
#   BASELINE=1 scripts/build.sh workspace release profile, no wasm-opt
#
# Needs: cargo with the wasm32-unknown-unknown target, wasm-bindgen-cli at the
# same version as the wasm-bindgen crate and Python fontTools for the font
# subsets. Optional: wasm-opt (binaryen).
set -euo pipefail

python="${PYTHON:-python3}"
if ! "$python" -c "import fontTools.subset" 2>/dev/null; then
  echo "error: $python has no fontTools; the font subsets need it." >&2
  echo "Install it (for example: python3 -m pip install fonttools, or your OS package" >&2
  echo "python3-fonttools), or point PYTHON at a Python that has it." >&2
  exit 1
fi

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

rm -rf "$out"
mkdir -p "$out"
for example in demo gallery; do
  cargo build --manifest-path "$workspace/Cargo.toml" -p astryx-web --example "$example" \
    --target wasm32-unknown-unknown --profile "$profile"
  wasm-bindgen --target web --no-typescript --out-dir "$out" \
    "$target_dir/wasm32-unknown-unknown/$profile/examples/$example.wasm"
  if [ "${BASELINE:-0}" != 1 ] && command -v wasm-opt >/dev/null; then
    # The features Rust enables by default for wasm32 must be allowed explicitly.
    wasm-opt -Oz --strip-debug --strip-producers \
      --enable-bulk-memory --enable-mutable-globals --enable-sign-ext \
      --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue \
      "$out/${example}_bg.wasm" -o "$out/${example}_bg.wasm.opt"
    mv "$out/${example}_bg.wasm.opt" "$out/${example}_bg.wasm"
  fi
done

cp "$crate/web/index.html" "$crate/web/gallery.html" "$out/"
"$python" "$crate/scripts/subset-fonts.py" "$out/fonts"

echo "built $out"
