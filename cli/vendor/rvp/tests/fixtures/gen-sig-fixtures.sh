#!/bin/sh
# Regenerates the signature fixtures for tests/verify.rs. Host-only, run inside the build
# image from the crate root:   ./t.sh sh tests/fixtures/gen-sig-fixtures.sh
#
# minisign 0.11: `-S` signs prehashed (algo "ED") by default; `-l` forces legacy "Ed".
# (`-H` only exists on `-V`, where it *requires* a prehashed signature.)
# Only public keys are committed; secret keys live in a throwaway temp dir.
set -eu
cd "$(dirname "$0")"
k=$(mktemp -d)
trap 'rm -rf "$k"' EXIT
tc="revised-approved 2026-11-12T00:00:00Z"

# pkg.core = the deterministic core of fixtures/pkg (written by an ignored test)
(cd ../.. && RVP_REGEN=1 cargo test -q --test verify -- --ignored regen_pkg_core >/dev/null)

minisign -G -W -p test.pub -s "$k/a.key" -f >/dev/null
minisign -G -W -p "$k/b.pub" -s "$k/b.key" >/dev/null

minisign -S -s "$k/a.key" -m pkg.core -x pkg.core.minisig -t "$tc" >/dev/null
minisign -S -s "$k/b.key" -m pkg.core -x pkg.core.otherkey.minisig -t "$tc" >/dev/null
minisign -S -l -s "$k/a.key" -m pkg.core -x pkg.core.legacy.minisig -t "$tc" >/dev/null

# sanity: real minisign accepts the good one in prehashed mode
minisign -V -H -q -p test.pub -m pkg.core -x pkg.core.minisig
echo "fixtures regenerated"
