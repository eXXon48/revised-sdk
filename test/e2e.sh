#!/bin/sh
# End-to-end check: new -> lint -> build (app and skin) -> preview up/eval/shot/down.
# Needs a container engine, the image (`revised-sdk setup`) and Opera in the volume.
# Running this script means accepting the Opera 12.10 EULA on this machine
# (it passes --accept-opera-license to `preview up`).
# Usage: REVISED_SDK=/path/to/revised-sdk sh test/e2e.sh   (default: revised-sdk on PATH)
# Stock leg (`preview up --stock`): set REVISED_BASELINE to an extracted stock rootfs (POSIX file
# system); its screenshots home.png, ent.png, list.png go to REVISED_SHOTS (default: a temp dir
# that is removed). REVISED_ENGINE = the engine binary used to inspect the container (podman).
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
run new-skin "$sdk" -C "$tmp" new skin e2e-skin
run lint-skin "$sdk" -C "$tmp/e2e-skin" lint --json; has '"ok": *true'
run build-skin "$sdk" -C "$tmp/e2e-skin" build --json; has '"ok": *true'
step=build-skin-artifact; ls "$tmp"/e2e-skin/dist/e2e-skin-0.1.0.rvp >/dev/null || exit 1
run preview-up "$sdk" preview up --accept-opera-license --json; has '"ok": *true'
run eval-api "$sdk" preview eval 'RevisedHMI.apiVersion' --json; has '"result": *"1"'
# a browser left open on :8750 could answer too: require the container Opera (X11)
run eval-ua "$sdk" preview eval 'navigator.userAgent' --json; has 'X11'
run shot "$sdk" preview shot s.png --json; has '"ok": *true'
step=shot-png
[ "$(od -An -tx1 -N8 s.png | tr -d ' \n')" = 89504e470d0a1a0a ] || exit 1
[ "$(od -An -tu1 -j16 -N8 s.png | tr -s ' ' | sed 's/^ //')" = "0 0 3 32 0 0 1 224" ] || exit 1
run preview-down "$sdk" preview down --json; has '"ok": *true'

# poll an eval until its JSON result matches (the stock GUI draws asynchronously)
until_eval() {
  step=$1; i=0
  while [ $i -lt 20 ]; do
    out=$("$sdk" preview eval "$2" --json) && echo "$out" | grep -Eq "$3" && return 0
    i=$((i+1)); sleep 0.5
  done
  echo "$out" >&2; exit 1
}

if [ -z "${REVISED_BASELINE:-}" ]; then
  echo "stock leg skipped: REVISED_BASELINE is not set"
else
  engine=${REVISED_ENGINE:-podman}
  shots=${REVISED_SHOTS:-$tmp/shots}
  mkdir -p "$shots" "$tmp/empty"
  step=stock-missing-gui
  out=$("$sdk" preview up --accept-opera-license --stock "$tmp/empty" --json) && { echo "$out" >&2; exit 1; }
  has 'no /jci/gui/index.html in the baseline'
  run stock-up "$sdk" preview up --accept-opera-license --stock "$REVISED_BASELINE" --json; has '"ok": *true'
  # the copy lives in the container: a file left behind by this run must not survive the next `up`
  step=stock-mark; "$engine" exec revised-sdk-preview touch /jci/gui/left-by-last-run || exit 1
  run stock-up-again "$sdk" preview up --accept-opera-license --stock "$REVISED_BASELINE" --json; has '"ok": *true'
  step=stock-fresh-copy; "$engine" exec revised-sdk-preview test ! -e /jci/gui/left-by-last-run || exit 1
  run stock-home "$sdk" preview eval 'framework.getCurrCtxtId()' --json; has '"result": *"HomeScreen"'
  run shot-home "$sdk" preview shot home.png --json; has '"ok": *true'
  run ctx-ent "$sdk" preview ctx system Entertainment --json; has '"ok": *true'
  until_eval ent 'framework.getCurrCtxtId()' '"result": *"Entertainment"'
  sleep 2
  run shot-ent "$sdk" preview shot ent.png --json; has '"ok": *true'
  run ctx-list "$sdk" preview ctx fmradio StationList --json; has '"ok": *true'
  until_eval list 'document.body.innerHTML.indexOf("Harbor FM") >= 0' '"result": *"true"'
  sleep 1
  run shot-list "$sdk" preview shot list.png --json; has '"ok": *true'
  step=ctx-bad-id
  out=$("$sdk" preview ctx fmradio ../x --json) && { echo "$out" >&2; exit 1; }
  has 'letters, digits or _'
  cp home.png ent.png list.png "$shots"/ || exit 1
  run stock-down "$sdk" preview down --json; has '"ok": *true'
  echo "stock leg: screenshots in $shots"
fi
echo "e2e PASS"
