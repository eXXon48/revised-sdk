#!/bin/sh
# Local HMI sim driver (DEV ONLY, no unit). The rvp-build container plays the unit:
# /data, /data_persist, /etc/cmu-re are bind mounts of local-sim/root/*.
#   u.sh up            pod rvp-pod (host 127.0.0.1 only): daemon rvp-sim (ws :2777, catalog
#                      http://127.0.0.1:8750/catalog/ = catalog-dev) + node http rvp-http :8750
#   u.sh down          stop the pod
#   u.sh reset         wipe sim state and reinstall the store + dev bridge + t-hmi-hello offline
#   u.sh js '<code>'   eval in the Opera page via cmu-re-dev-bridge (installed by reset), print the result
#   u.sh ctl '<line>'  raw line to the daemon ctl FIFO (open | <keycode>)
#   u.sh log           daemon log tail
#   u.sh opera         launch desktop Opera 12.10 on the sim page
export MSYS_NO_PATHCONV=1
[ -f "$(dirname "$0")/local.env" ] && . "$(dirname "$0")/local.env"   # OPERA (git-ignored)
D=$(cd "$(dirname "$0")" && pwd)
R=$(cd "$D" && { pwd -W 2>/dev/null || pwd; })
V="-v $R:/sim -v $R/root/data:/data -v $R/root/data_persist:/data_persist -v $R/root/etc-cmu-re:/etc/cmu-re -v rvp-target:/target"
# platform-package targets (cmu-re-sys-time/-trust): stub SM configs + boot hook + empty ssl dirs, recreated by reset
V="$V -v $R/root/jci-sm:/jci/sm -v $R/root/etc-ssl-certs:/etc/ssl/certs -v $R/root/usr-lib-ssl:/usr/lib/ssl -v $R/root/jci-scripts:/jci/scripts"
BIN=/target/release/revised-daemon
case "$1" in
up)
  podman pod rm -f rvp-pod >/dev/null 2>&1; podman rm -f rvp-sim >/dev/null 2>&1
  # the local api stack (cmu-revised-deployment local/up.sh) shares network revised-local: the daemon
  # reaches it as http://revised-local:8080 (that pod name resolves on the network)
  net=; podman network exists revised-local 2>/dev/null && net="--network revised-local"
  podman pod create --name rvp-pod $net -p 127.0.0.1:2777:2777 -p 127.0.0.1:8750:8750 >/dev/null
  podman run -d --pod rvp-pod --name rvp-http -v "$R":/sim -v "$R/../catalog-dev":/catalog-dev:ro \
    -e ROOT=/sim/root -e CATALOG=/catalog-dev docker.io/library/node:22-alpine node /sim/serve.js >/dev/null
  podman run -d --pod rvp-pod --name rvp-sim $V rvp-build \
    $BIN --listen 0.0.0.0:2777 --ctl /tmp/rvp-ctl --catalog http://revised-local:8080/v1 --dl http://127.0.0.1:8750/catalog --vin JMZTEST0000000001 --wlan eth0 >/dev/null
  sleep 2; podman logs rvp-http; podman logs rvp-sim 2>&1 | tail -3 ;;
down)
  podman pod rm -f rvp-pod >/dev/null 2>&1; echo "pod down" ;;
reset)
  podman pod rm -f rvp-pod >/dev/null 2>&1
  rm -rf "$D/root/jci-sm" "$D/root/jci-scripts" "$D/root/etc-ssl-certs" "$D/root/usr-lib-ssl"
  mkdir -p "$D/root/jci-sm" "$D/root/jci-scripts" "$D/root/etc-ssl-certs" "$D/root/usr-lib-ssl"
  cp "$D/sm-stub.conf" "$D/root/jci-sm/sm.conf"; cp "$D/sm-stub.conf" "$D/root/jci-sm/sm_WCP.conf"; cp "$D/normal-mode-stub.sh" "$D/root/jci-scripts/start_normal_mode.sh"
  # the sed: offline install skips the catalog, so dev-bridge gets the devOnly mark a devmode install would
  # set (it then shows only under Developer)
  podman run --rm $V -v "$R/../catalog-dev":/catalog-dev:ro rvp-build sh -c "rm -rf /data_persist/cmu-re /data/integration/*; \
    mkdir -p /data_persist/cmu-re/store && md5sum /jci/sm/sm.conf /jci/sm/sm_WCP.conf /jci/scripts/start_normal_mode.sh > /data_persist/cmu-re/store/stock.md5; cd /sim && \
    $BIN --install-file /catalog-dev/pkgs/cmu-re-sys-time-1.2.0.rvp --keys /catalog-dev/keys.json && \
    $BIN --install-file /catalog-dev/pkgs/cmu-re-sys-trust-1.0.0.rvp --keys /catalog-dev/keys.json && \
    $BIN --install-file /catalog-dev/pkgs/cmu-re-store-0.6.0.rvp --keys /catalog-dev/keys.json && \
    $BIN --install-file /catalog-dev/pkgs/cmu-re-dev-bridge-1.0.0.rvp --keys /catalog-dev/keys.json && \
    sed -i 's/\"name\": \"cmu-re-dev-bridge\",/&\"dev_only\":true,/' /data_persist/cmu-re/pkg/.state/installed.json && \
    $BIN --install-file /catalog-dev/pkgs/t-hmi-hello-1.0.0.rvp --keys /catalog-dev/keys.json" ;;
js)
  code=$(printf '%s' "$2" | base64 -w0)
  n=$(podman logs rvp-sim 2>&1 | grep -c '"eval"')
  podman exec rvp-sim sh -c "echo 'js:eval(atob(\"$code\"))' > /tmp/rvp-ctl"
  i=0; while [ $i -lt 30 ]; do
    [ "$(podman logs rvp-sim 2>&1 | grep -c '"eval"')" -gt "$n" ] && break; sleep 0.2; i=$((i+1)); done
  if [ $i -lt 30 ]; then podman logs rvp-sim 2>&1 | grep '"eval"' | tail -1 | sed 's/.*"action"://'
  else echo "no reply (dev bridge installed? HMI page started?)"; fi ;;
ctl) podman exec rvp-sim sh -c "echo '$2' > /tmp/rvp-ctl" ;;
log) podman logs --tail "${2:-30}" rvp-sim 2>&1 ;;
opera) ("${OPERA:?set OPERA=<path to opera.exe> in local.env}" "http://127.0.0.1:8750/jci/gui/index.html" >/dev/null 2>&1 &) ;;
*) sed -n 2,11p "$0"; exit 1 ;;
esac
