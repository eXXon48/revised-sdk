#!/bin/sh
# Container entrypoint of `revised-sdk preview up` (fake unit; DEV ONLY).
# env: RVP = the built package inside the container (/work/dist/<name>-<version>.rvp)
#      VNC=1: live view of the screen (x11vnc on :5900, no password; the host publishes it on loopback)
#      STOCK=1: run the stock GUI from the baseline mounted read-only at /stock, with stub sockets
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
if [ "${STOCK:-0}" = 1 ]; then
  if [ ! -f /stock/jci/gui/index.html ]; then
    echo "[preview] no /jci/gui/index.html in the baseline" >&2
    exit 3
  fi
  # a copy in this container's writable layer: packages change it, the baseline stays untouched,
  # and the next `preview up` (a new container) starts from a fresh copy
  mkdir -p /jci && cp -a /stock/jci/gui /jci/gui
  # the daemon accepts a change to a stock file only over the published stock md5
  grep -h '  /jci/gui/' /opt/revised/baseline/*.md5 > $SIM/stock.md5
  ln -sfn /jci /web/jci
  node $P/stock/stub.js &
else
  ln -sfn $P/stub/jci /web/jci
fi
ln -sfn $P/stub/preview /web/preview
ln -sfn /data /web/data
ln -sfn $SIM /web/sim
ln -sfn $SIM/cmu-re /web/data_persist/cmu-re
cp $P/stub/data/integration/developer_mode_GuiConfig.js /data/integration/
# the daemon relays the ctl FIFO (eval, keys) only while cmu-re-dev-bridge is installed; the stub
# injector evals by itself, so an empty jail file stands in for the package
mkdir -p $SIM/cmu-re/pkg/cmu-re-dev-bridge/ui && : > $SIM/cmu-re/pkg/cmu-re-dev-bridge/ui/bridge.js

# The daemon asks the api to confirm every unsigned install: api-stub.js stands in for it.
# --log-frames on: `preview eval` reads the replies from the daemon log.
node $P/api-stub.js &
revised-daemon --test-root $SIM --listen 0.0.0.0:2777 --ctl /tmp/rvp-ctl --wlan eth0 --catalog http://127.0.0.1:8770/v1 --log-frames on &
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

# Opera 12.10 has no kiosk mode on Linux and fullscreen needs a window manager. Instead the X screen is
# taller than the HMI and the Opera window hangs above the top edge, so its tab/address bars are off
# screen and the page viewport is exactly 800x480 at (0,0). 61 = chrome above the page, 22 = status bar
# below it (Opera 12.10 default skin); `shot` crops to 800x480.
Xvfb :99 -screen 0 800x563x24 -nolisten tcp &
if [ "${VNC:-0}" = 1 ]; then
  # no -viewonly: mouse and keyboard reach Opera
  (sleep 1; x11vnc -display :99 -forever -shared -nopw -rfbport 5900 -quiet) &
fi
if [ "${START_OPERA:-1}" = 1 ]; then
  # START_OPERA=1 means the developer accepted Opera's license on the command line
  # (`preview up --accept-opera-license`); this only records it in the throwaway profile.
  printf '[State]\nAccept License=1\n' > /tmp/operaprofile/operaprefs.ini
  sleep 1
  DISPLAY=:99 OPERA_PERSONALDIR=/tmp/operaprofile \
    /opera/*/opera -nosession -geometry 800x563+0-61 \
    http://127.0.0.1:8750/jci/gui/index.html &
fi
wait
