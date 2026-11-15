#!/bin/sh
# Run a command in the rvp build image against cli/.
# Usage: cli/t.sh cargo test      (target dir + cargo registry live in podman volumes)
export MSYS_NO_PATHCONV=1
here=$(cd "$(dirname "$0")" && { pwd -W 2>/dev/null || pwd; })
exec podman run --rm -v "$here":/work -w /work \
  -v sdk-target:/target -e CARGO_TARGET_DIR=/target \
  -v sdk-cargo-reg:/root/.cargo/registry -v rvp-rustup:/root/.rustup rvp-build "$@"
