#!/bin/sh
# Assemble ../../catalog-dev/store-src (manifest + payload) and build the dev catalog (host, git-bash).
# Build the daemon first (see rvp ./t.sh cargo zigbuild ... -> revised-daemon-arm here).
set -eu
cd "$(dirname "$0")"
src=../../catalog-dev/store-src
rm -rf "$src"
mkdir -p "$src/payload/bin" "$src/payload/ui"
cp store-manifest.json "$src/manifest.json"
cp revised-daemon-arm "$src/payload/bin/revised-daemon"
cp controls.js "$src/payload/ui/controls.js"
cp sdk.js "$src/payload/ui/sdk.js"
cp hmi.json "$src/payload/ui/hmi.json"
cp developer_mode_GuiConfig.js "$src/payload/developer_mode_GuiConfig.js"
cd ../.. && ./t.sh sh daemon/testpkgs/mk-catalog.sh
# bundle copy for run-on-unit.sh (wants exactly one cmu-re-store-*.rvp next to it)
cd ui/deploy
rm -f cmu-re-store-*.rvp
cp ../../catalog-dev/pkgs/cmu-re-store-*.rvp .
