#!/bin/sh
# Run a command in the rvp build image against cli/ (whole sdk root mounted: cli/ includes ../container).
# Panic locations in the built binary then read `./cli/vendor/...`, not the build container's paths.
# Usage: cli/t.sh cargo test      (target dir + cargo registry live in podman volumes)
export MSYS_NO_PATHCONV=1
here=$(cd "$(dirname "$0")/.." && { pwd -W 2>/dev/null || pwd; })
exec podman run --rm -v "$here":/work -w /work/cli \
  -v sdk-target:/target -e CARGO_TARGET_DIR=/target \
  -e RUSTFLAGS="--remap-path-prefix=/work=. --remap-path-prefix=/root/.cargo=cargo" \
  -v sdk-cargo-reg:/root/.cargo/registry -v rvp-rustup:/root/.rustup rvp-build "$@"
