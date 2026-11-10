#!/bin/sh
# Link the two Astryx packages where every app below finds them. Contract resolves a package by
# searching node_modules upward from the importing file's real path, so one link here serves
# the apps and lets @astryx-spike/astryx-layers find the library. (A `file:` install copies the
# package, which leaves stale copies after each edit.)
set -e
cd "$(dirname "$0")/.."
mkdir -p node_modules/@astryx-spike
ln -sfn ../../astryx node_modules/@astryx-spike/astryx
ln -sfn ../../astryx-layers node_modules/@astryx-spike/astryx-layers
