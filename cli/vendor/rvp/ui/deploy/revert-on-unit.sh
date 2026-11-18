#!/bin/sh
# Revised Store dev REVERT on the CMU150.  *** RUN ON THE UNIT ***
# Undoes run-on-unit.sh: stops the daemon, then the daemon's own offline
# --uninstall-pkg cmu-re-store removes the injector, its 'A f' index row, the jail
# and the registry entry; then cmu-re-sys-trust and cmu-re-sys-time (SM configs back to
# stock). Refuses while other packages are installed.
# Run from the scp'd bundle (needs revised-daemon-arm next to it).
# /data_persist/cmu-re/store state and vanilla are LEFT in place (tripwire contract).
# No rootfs edit, no reboot.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)

echo ">> stop daemon"
kill $(pidof revised-daemon) 2>/dev/null || true
i=0
while [ -n "$(pidof revised-daemon)" ] && [ "$i" -lt 25 ]; do sleep 0.2 2>/dev/null || sleep 1; i=$((i + 1)); done
if [ -n "$(pidof revised-daemon)" ]; then
  echo "revised-daemon still running after ~5 s; not uninstalling. Stop it, then re-run."
  exit 1
fi

# the store (0.6.0+) depends on the platform packages and goes first; an older store without
# depends is refused until they are gone: retry the refused ones until a pass makes no progress
left="cmu-re-store cmu-re-sys-trust cmu-re-sys-time"
while [ -n "$left" ]; do
  next=
  for p in $left; do
    echo ">> uninstall $p"
    "$HERE/revised-daemon-arm" --uninstall-pkg "$p" || next="$next $p"
  done
  if [ "$(echo $next)" = "$left" ]; then
    echo "uninstall refused for:$next (other packages installed? uninstall them from the UI first, then re-run)."
    exit 1
  fi
  left=$(echo $next)
done

rm -f /tmp/rvp-ctl /tmp/revised-daemon.log

echo ">> done. SafeReboot to drop the injector from the running HMI."
