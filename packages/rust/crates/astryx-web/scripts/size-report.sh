#!/usr/bin/env bash
# Prints raw, gzip -9, and brotli sizes of a built dist directory.
#   scripts/size-report.sh [dist]
set -euo pipefail
dist="${1:-$(cd "$(dirname "$0")/.." && pwd)/dist}"

gz() { gzip -9 -c "$1" | wc -c; }
br() { if command -v brotli >/dev/null; then brotli -q 11 -c "$1" | wc -c; else echo "-"; fi; }
row() { printf "%-28s %10s %10s %10s\n" "$1" "$2" "$3" "$4"; }

row file raw gzip brotli
total_raw=0; total_gz=0
for f in demo_bg.wasm demo.js index.html; do
  raw=$(wc -c < "$dist/$f"); g=$(gz "$dist/$f")
  total_raw=$((total_raw + raw)); total_gz=$((total_gz + g))
  row "$f" "$raw" "$g" "$(br "$dist/$f")"
done
row "app total (wasm+js+html)" "$total_raw" "$total_gz" ""
# Runtime subsets: the first frame needs the Latin ones; others load on demand.
for group in latin all; do
  raw_sum=0; gz_sum=0
  for f in "$dist"/fonts/*.ttf; do
    case "$group:$(basename "$f")" in latin:*.latin.ttf|all:*) ;; *) continue ;; esac
    raw=$(wc -c < "$f"); g=$(gz "$f")
    raw_sum=$((raw_sum + raw)); gz_sum=$((gz_sum + g))
  done
  row "font subsets ($group)" "$raw_sum" "$gz_sum" ""
done

# The renderer still embeds these fonts in the wasm; listed for reference.
fonts="$(cd "$(dirname "$0")/../../.." && pwd)/assets/fonts"
font_raw=0; font_gz=0
for f in "$fonts"/*.ttf; do
  raw=$(wc -c < "$f"); g=$(gz "$f")
  font_raw=$((font_raw + raw)); font_gz=$((font_gz + g))
done
row "embedded fonts (in wasm)" "$font_raw" "$font_gz" ""
