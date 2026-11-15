#!/bin/sh
# Local SDK sim: container = fake unit (DEV ONLY). Usage: sh local-sim/run.sh <cmd...>
export MSYS_NO_PATHCONV=1
exec podman run --rm "$@"
