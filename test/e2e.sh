#!/bin/sh
# End-to-end check: new -> lint -> build -> preview up/eval/shot/down.
# Needs a container engine, the image (`revised-sdk setup`) and Opera in the volume.
# Running this script means accepting the Opera 12.10 EULA on this machine
# (it passes --accept-opera-license to `preview up`).
# Usage: REVISED_SDK=/path/to/revised-sdk sh test/e2e.sh   (default: revised-sdk on PATH)
sdk=${REVISED_SDK:-revised-sdk}
tmp=$(mktemp -d) || exit 2
step=start
cleanup() {
  rc=$?
  (cd "$tmp" && "$sdk" preview down >/dev/null 2>&1)
  rm -rf "$tmp"
  [ $rc -eq 0 ] || echo "e2e FAILED at step: $step" >&2
  exit $rc
}
trap cleanup EXIT
trap 'exit 130' INT TERM
cd "$tmp" || exit 2

run() { step=$1; shift; out=$("$@") || { echo "$out" >&2; exit 1; }; }
has() { echo "$out" | grep -Eq "$1" || { echo "$out" >&2; exit 1; }; }

run new "$sdk" new app e2e
cd e2e || exit 2
run lint "$sdk" lint --json;  has '"ok": *true'
run build "$sdk" build --json; has '"ok": *true'
step=build-artifact; ls dist/*.rvp >/dev/null || exit 1
run preview-up "$sdk" preview up --accept-opera-license --json; has '"ok": *true'
run eval-api "$sdk" preview eval 'RevisedHMI.apiVersion' --json; has '"result": *"1"'
# a browser left open on :8750 could answer too: require the container Opera (X11)
run eval-ua "$sdk" preview eval 'navigator.userAgent' --json; has 'X11'
run shot "$sdk" preview shot s.png --json; has '"ok": *true'
step=shot-png
[ "$(od -An -tx1 -N8 s.png | tr -d ' \n')" = 89504e470d0a1a0a ] || exit 1
[ "$(od -An -tu1 -j16 -N8 s.png | tr -s ' ' | sed 's/^ //')" = "0 0 3 32 0 0 1 224" ] || exit 1
run preview-down "$sdk" preview down --json; has '"ok": *true'
echo "e2e PASS"
