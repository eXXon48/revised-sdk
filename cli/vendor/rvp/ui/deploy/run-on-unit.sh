#!/bin/sh
# Revised Store dev deploy on the CMU150 — installs the store AS A PACKAGE (cmu-re-store)
# via the daemon's own offline self-install.  *** RUN ON THE UNIT ***
#
# Requires on rootfs (placed by hand, NOT touched here, no remount): /etc/cmu-re/tripwire.cnf
# (contract_version=1) and the /cmu-re/t2 fixtures. See the App Deploy Runbook.
#
# Touches ONLY (all reversible with revert-on-unit.sh, no rootfs edit, no reboot):
#   /data_persist/cmu-re/store/   t2-stock.paths + stock.md5 (fresh only), pubkey (after verified install)
#   /data_persist/cmu-re/pkg/cmu-re-store/   jail: bin/revised-daemon, ui/controls.js
#   /data_persist/cmu-re/vanilla/index       'A f' row for the injector
#   /data/integration/developer_mode_GuiConfig.js   injector, written BY the package install
#   /tmp/rvp-ctl, /tmp/revised-daemon.log    dev FIFO + daemon log
# Re-run on an installed store = upgrade in place. Stop the daemon first.
#
# Bundle (scp the ui/deploy dir, e.g. /tmp/rvp-spike) must hold: revised-daemon-arm,
# exactly one cmu-re-store-*.rvp (mk-store-pkg.sh drops it here), s4/pubkey.minisign,
# s4/t2/t2-stock.paths, s4/t2/stock.md5.
#   ssh ... 'CATALOG=http://<dev-pc>:8740/ sh /tmp/rvp-spike/run-on-unit.sh'
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ST=/data_persist/cmu-re/store
JAIL=/data_persist/cmu-re/pkg/cmu-re-store
CATALOG=${CATALOG:?set CATALOG=http://<dev-pc>:8740/}
# dev-only: pause after each progress frame so the UI stages are readable (0 = off)
DELAY=${RVP_STAGE_DELAY_MS:-300}
DLOG=/tmp/revised-daemon.log

echo ">> stop old ws-logger spike; refuse if the daemon runs"
kill $(pidof rvp-ws-logger) 2>/dev/null || true
if [ -n "$(pidof revised-daemon)" ]; then
  echo "stop the daemon first: kill $(pidof revised-daemon)"
  exit 1
fi

echo ">> find package"
set -- "$HERE"/cmu-re-store-*.rvp
if [ $# -ne 1 ] || [ ! -f "$1" ]; then
  echo "need exactly one cmu-re-store-*.rvp in $HERE, found: $*"
  exit 1
fi
RVP=$1
echo "$RVP"

echo ">> state dir $ST (fresh only: keeps state of an earlier run)"
mkdir -p "$ST"
if [ ! -e "$ST/t2-stock.paths" ]; then
  cp "$HERE/s4/t2/t2-stock.paths" "$HERE/s4/t2/stock.md5" "$ST/"
fi
chown -R 1000:1000 /data_persist/cmu-re/store

echo ">> self-install $RVP"
"$HERE/revised-daemon-arm" --install-file "$RVP" --pubkey "$HERE/s4/pubkey.minisign"

echo ">> start jail daemon (catalog $CATALOG)"
RVP_STAGE_DELAY_MS=$DELAY nohup "$JAIL/bin/revised-daemon" --listen 127.0.0.1:2777 --catalog "$CATALOG" \
  --extra-stock "$ST/t2-stock.paths" --ctl /tmp/rvp-ctl > "$DLOG" 2>&1 < /dev/null &
sleep 2
echo ">> daemon pid(s): $(pidof revised-daemon)"
cat "$DLOG"

echo ">> injector row in the vanilla index (expect exactly one)"
# canonical live path: /data is a symlink to /tmp/mnt/data
grep 'data/integration/developer_mode_GuiConfig.js' /data_persist/cmu-re/vanilla/index

echo
echo "NEXT (not done here): SafeReboot if the injector is new (runbook); open Apps -> Revised Store."
echo "Revert: sh $HERE/revert-on-unit.sh"
