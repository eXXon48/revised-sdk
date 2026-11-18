#!/bin/sh
# Assemble ../../catalog-dev/store-src (manifest + payload) and build the dev catalog (host, git-bash).
# STORE_SRC_ONLY=1: stop after assembly (no dev catalog build).
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
# dev-only eval bridge from the sibling repo (skipped if it is not checked out)
bsrc=../../catalog-dev/dev-bridge-src
rm -rf "$bsrc"
if [ -f ../../../cmu-re-dev-bridge/pkg/manifest.json ]; then
    cp -r ../../../cmu-re-dev-bridge/pkg "$bsrc"
    sed -i 's/"version": "0.0.0"/"version": "1.0.0"/' "$bsrc/manifest.json"
    printf '"devOnly": true
' >"$bsrc/catalog.frag"
else
    echo "warning: ../cmu-re-dev-bridge not checked out: no dev bridge in the dev catalog" >&2
fi
# release CI packs + signs the assembled tree itself
[ "${STORE_SRC_ONLY:-}" = 1 ] && exit 0
# platform packages the store depends on, from the sibling repos (required)
for p in sys-time sys-trust; do
    psrc=../../catalog-dev/$p-src
    rm -rf "$psrc"
    [ -f ../../../cmu-re-$p/pkg/manifest.json ] || { echo "../cmu-re-$p not checked out (the store depends on it)" >&2; exit 1; }
    cp -r ../../../cmu-re-$p/pkg "$psrc"
    case $p in sys-time) v=1.2.0 ;; *) v=1.0.0 ;; esac   # dev label; release CI takes it from the tag
    sed -i "s/\"version\": \"0.0.0\"/\"version\": \"$v\"/" "$psrc/manifest.json"
done
# sys-time ships a built library (sh tools/build-timefix.sh in that repo, rvp-build image)
cp ../../../cmu-re-sys-time/build/libcmure-timefix.so ../../catalog-dev/sys-time-src/payload/
cd ../.. && ./t.sh sh daemon/testpkgs/mk-catalog.sh
# bundle copy for run-on-unit.sh (wants exactly one rvp per package next to it + the stock reference)
cd ui/deploy
rm -f cmu-re-store-*.rvp cmu-re-sys-time-*.rvp cmu-re-sys-trust-*.rvp
cp ../../catalog-dev/pkgs/cmu-re-store-*.rvp ../../catalog-dev/pkgs/cmu-re-sys-time-*.rvp \
   ../../catalog-dev/pkgs/cmu-re-sys-trust-*.rvp .
cp ../../catalog-dev/stockref/74.00.324A.md5 s4/stockref.md5
cp ../../catalog-dev/keys.json ../../catalog-dev/keys.json.minisig s4/
