#!/bin/sh
# Run a command in the rvp build image against this checkout.
# Usage: ./t.sh cargo test        (target dir + cargo registry live in podman volumes)
# ./catalog-dev (the signed dev catalog, in this repo) is mounted at /catalog-dev.
export MSYS_NO_PATHCONV=1
mkdir -p ./catalog-dev
cat_dev=$(cd ./catalog-dev && { pwd -W 2>/dev/null || pwd; })
# dev root pubkey is embedded in the daemon build (daemon/build.rs); absent on a fresh checkout = rvp-only work
root=; [ -f ./catalog-dev/root.pub ] && root="-e REVISED_ROOT_PUB=/catalog-dev/root.pub"
exec podman run --rm $root -v "$(pwd -W 2>/dev/null || pwd)":/work -w /work -v "$cat_dev":/catalog-dev \
  -v rvp-target:/target -e CARGO_TARGET_DIR=/target \
  -v rvp-cargo-reg:/root/.cargo/registry -v rvp-rustup:/root/.rustup rvp-build "$@"
