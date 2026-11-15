#!/bin/sh
# Container entrypoint of `revised-sdk preview up` (fake unit; DEV ONLY).
# env: RVP = the built package inside the container (/work/dist/<name>-<version>.rvp)
# Order matters: the HMI SDK reads its registry at page load, so the package is installed
# before Opera starts.
set -u
trap "exit 0" TERM INT   # PID 1: without this `docker rm -f` waits 10 s for SIGKILL
P=/opt/revised/preview
SIM=/sim
mkdir -p $SIM/tmp/revised-push $SIM/cmu-re/store /data/integration /web/data_persist /tmp/operaprofile

# Developer mode + a linked account, seeded directly (a fake unit has no pairing server).
: > $SIM/devmode
printf '{"token":"preview","name":"sdk-preview"}' > $SIM/cmu-re/store/account.json
cp $P/stub/pubkey.minisign $SIM/cmu-re/store/pubkey.minisign
: > $SIM/test-stock.paths

# One web root for the page, the injector, the registry and the package jails.
ln -sfn $P/stub/jci /web/jci
ln -sfn $P/stub/preview /web/preview
ln -sfn /data /web/data
ln -sfn $SIM/cmu-re/ui /web/data_persist/ui
ln -sfn $SIM /web/sim
ln -sfn $SIM/cmu-re /web/data_persist/cmu-re
cp $P/stub/data/integration/developer_mode_GuiConfig.js /data/integration/

revised-daemon --test-root $SIM --listen 0.0.0.0:2777 --ctl /tmp/rvp-ctl --dev-pair on --wlan eth0 &
ROOT=/web CATALOG=/nonexistent node /opt/revised/sim/serve.js &

# install-push: copy into the push dir, ask the daemon, delete the copy either way
f=$SIM/tmp/revised-push/$(basename "$RVP")
cp "$RVP" "$f"
i=0; r=
while [ $i -lt 30 ]; do
  r=$(node $P/wsclient.js 2777 "{\"control\":\"store\",\"action\":\"install-push\",\"value\":{\"path\":\"$f\"}}" '"op":"install-push"' 2>&1) && break
  i=$((i+1)); sleep 0.5
done
rm -f "$f"
case "$r" in
  *'"ok":true'*) echo "[preview] installed $(basename "$RVP")" ;;
  *) echo "[preview] install-push failed: $r" >&2; exit 3 ;;
esac

Xvfb :99 -screen 0 800x480x24 -nolisten tcp &
if [ "${START_OPERA:-1}" = 1 ]; then
  # START_OPERA=1 means the developer accepted Opera's license on the command line
  # (`preview up --accept-opera-license`); this only records it in the throwaway profile.
  printf '[State]\nAccept License=1\n' > /tmp/operaprofile/operaprefs.ini
  sleep 1
  DISPLAY=:99 OPERA_PERSONALDIR=/tmp/operaprofile \
    /opera/*/opera -nosession -geometry 800x480+0+0 -fullscreen \
    http://127.0.0.1:8750/jci/gui/index.html &
fi
wait
