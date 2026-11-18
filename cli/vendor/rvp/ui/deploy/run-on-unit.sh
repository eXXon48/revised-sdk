#!/bin/sh
# Revised Store dev deploy on the CMU150 — installs the store AS A PACKAGE (cmu-re-store)
# via the daemon's own offline self-install.  *** RUN ON THE UNIT ***
#
# Requires on rootfs (placed by hand, NOT touched here, no remount): /etc/cmu-re/tripwire.cnf
# (contract_version=1 or 2) and the /cmu-re/t2 fixtures. See the App Deploy Runbook.
#
# Touches ONLY (all reversible with revert-on-unit.sh, no rootfs edit, no reboot):
#   /data_persist/cmu-re/store/   t2-stock.paths + stock.md5 (fresh only), keys.cache (after verified install)
#   /data_persist/cmu-re/pkg/cmu-re-store/   jail: bin/revised-daemon, ui/controls.js
#   /data_persist/cmu-re/vanilla/index       'A f' row for the injector + the platform packages' rows
#   cmu-re-sys-time: /jci/scripts/start_normal_mode.sh (one hook line; clock set from RTC next boot)
#   cmu-re-sys-trust: /etc/ssl/certs/ca-certificates.crt, /usr/lib/ssl/cert.pem (new files)
#   /data/integration/developer_mode_GuiConfig.js   injector, written BY the package install
#   /tmp/rvp-ctl, /tmp/revised-daemon.log    dev FIFO + daemon log
# Re-run on an installed store = upgrade in place. Stop the daemon first.
#
# Bundle (scp the ui/deploy dir, e.g. /tmp/rvp-spike) must hold: revised-daemon-arm,
# exactly one each of cmu-re-sys-time-*.rvp, cmu-re-sys-trust-*.rvp, cmu-re-store-*.rvp and
# s4/stockref.md5 (mk-store-pkg.sh drops them here), s4/keys.json + s4/keys.json.minisig,
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

echo ">> find packages (the store depends on the two platform packages: install order)"
RVPS=
for p in cmu-re-sys-time cmu-re-sys-trust cmu-re-store; do
  set -- "$HERE"/$p-[0-9]*.rvp
  if [ $# -ne 1 ] || [ ! -f "$1" ]; then
    echo "need exactly one $p-*.rvp in $HERE, found: $*"
    exit 1
  fi
  RVPS="$RVPS $1"
done
echo "$RVPS"

echo ">> state dir $ST (fresh only: keeps state of an earlier run)"
mkdir -p "$ST"
if [ ! -e "$ST/t2-stock.paths" ]; then
  cp "$HERE/s4/t2/t2-stock.paths" "$HERE/s4/t2/stock.md5" "$ST/"
fi
echo ">> stock.md5: add stock reference rows for paths it lacks (sys-time edits the SM configs)"
touch "$ST/stock.md5"
awk 'FILENAME == ARGV[1] { s[$2] = 1; next } NF >= 2 && !($2 in s)' "$ST/stock.md5" "$HERE/s4/stockref.md5" > "$ST/stock.md5.add"
echo "  + $(wc -l < "$ST/stock.md5.add") rows"
cat "$ST/stock.md5.add" >> "$ST/stock.md5" && rm -f "$ST/stock.md5.add"
chown -R 1000:1000 /data_persist/cmu-re/store

for RVP in $RVPS; do
  # skip a package already at this version: reinstalling a platform package uninstalls it
  # first, which the store's depends refuses
  n=${RVP##*/}; v=${n##*-}; v=${v%.rvp}; n=${n%-*}
  REG=/data_persist/cmu-re/pkg/.state/installed.json; [ -f "$REG" ] || REG=$ST/installed.json
  if grep -A1 "\"name\": \"$n\"" "$REG" 2>/dev/null | grep -q "\"version\": \"$v\""; then
    echo ">> $n $v already installed; skipped"; continue
  fi
  echo ">> self-install $RVP"
  "$HERE/revised-daemon-arm" --install-file "$RVP" --keys "$HERE/s4/keys.json"
done

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
