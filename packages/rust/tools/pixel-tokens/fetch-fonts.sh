#!/bin/sh
# Fetches the pixel theme's fallback fonts, Galmuri7 and Galmuri9 (SIL OFL
# 1.1), from the `galmuri` npm package at a pinned version and checks them.
# They stay a build input; nothing here is committed.
#
#   tools/pixel-tokens/fetch-fonts.sh [dir]
#
# Writes Galmuri7.ttf, Galmuri9.ttf, and OFL.txt to `dir`, else to
# $PIXEL_FONTS_DIR, else to target/pixel-fonts. Point PIXEL_FONTS_DIR at
# the same folder for the web build, and load the files with
# `add_fallback_font` in desktop and handheld apps.
set -eu

VERSION=2.40.3
URL="https://registry.npmjs.org/galmuri/-/galmuri-$VERSION.tgz"
TARBALL_SHA256=7d6b2b8534d157dd5e4eb8e01145f7388c0383bed8a96f5ec719eaacf1f86816
GALMURI7_SHA256=3882bd35066c26b0392cd4963ff9b3c151041dec34adc9d5633d137d1d9b9855
GALMURI9_SHA256=5cb68052ee0a15571747e91c20f145e24b51bb459c6cd58226fafee78d9c0b16

root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-${PIXEL_FONTS_DIR:-$root/target/pixel-fonts}}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

check() {
    got=$(sha256 "$1")
    if [ "$got" != "$2" ]; then
        echo "fetch-fonts: $1 has SHA-256 $got, expected $2" >&2
        exit 1
    fi
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl -fsSL "$URL" -o "$work/galmuri.tgz"
check "$work/galmuri.tgz" "$TARBALL_SHA256"
tar -xzf "$work/galmuri.tgz" -C "$work" package/dist/Galmuri7.ttf package/dist/Galmuri9.ttf package/ofl.md
check "$work/package/dist/Galmuri7.ttf" "$GALMURI7_SHA256"
check "$work/package/dist/Galmuri9.ttf" "$GALMURI9_SHA256"

mkdir -p "$out"
cp "$work/package/dist/Galmuri7.ttf" "$work/package/dist/Galmuri9.ttf" "$out/"
cp "$work/package/ofl.md" "$out/OFL.txt"
echo "fetch-fonts: Galmuri $VERSION in $out"
echo "export PIXEL_FONTS_DIR=$out"
