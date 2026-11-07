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
for f in gallery_bg.wasm gallery.js; do
  row "$f" "$(wc -c < "$dist/$f")" "$(gz "$dist/$f")" "$(br "$dist/$f")"
done
# Runtime subsets: the first frame needs the Latin ones; others load on demand.
for group in latin all; do
  raw_sum=0; gz_sum=0
  for f in "$dist"/fonts/*.ttf; do
    case "$group:$(basename "$f")" in latin:latin-[0-9]*.ttf|all:*) ;; *) continue ;; esac
    raw=$(wc -c < "$f"); g=$(gz "$f")
    raw_sum=$((raw_sum + raw)); gz_sum=$((gz_sum + g))
  done
  row "font subsets ($group)" "$raw_sum" "$gz_sum" ""
done

