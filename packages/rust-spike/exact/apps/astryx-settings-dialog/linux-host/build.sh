#!/bin/sh
# Build the app for the Linux host. Uses Exact's lockfile so the build resolves offline to the
# crates the Exact checkout already fetched. Prints the binary's path.
set -e
cd "$(dirname "$0")"
cp ../../../.exact-sdk/Cargo.lock Cargo.lock
cargo build --profile host-dev -p astryx-settings-dialog-linux --offline >&2
echo "$PWD/target/host-dev/astryx-settings-dialog-linux"
