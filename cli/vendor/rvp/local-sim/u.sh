#!/bin/sh
# Local HMI sim driver (DEV ONLY, no unit). The rvp-build container plays the unit:
# /data, /data_persist, /etc/cmu-re are bind mounts of local-sim/root/*.
#   u.sh up            pod rvp-pod (host 127.0.0.1 only): daemon rvp-sim (ws :2777, catalog
#                      http://127.0.0.1:8750/catalog/ = catalog-dev) + node http rvp-http :8750
#   u.sh down          stop the pod
#   u.sh reset         wipe sim state and reinstall the store + t-hmi-hello offline
#   u.sh js '<code>'   eval in the Opera page via the dev bridge, print the result
#   u.sh ctl '<line>'  raw line to the daemon ctl FIFO (open | <keycode>)
#   u.sh log           daemon log tail
#   u.sh opera         launch desktop Opera 12.10 on the sim page
export MSYS_NO_PATHCONV=1
[ -f "$(dirname "$0")/local.env" ] && . "$(dirname "$0")/local.env"   # OPERA (git-ignored)
D=$(cd "$(dirname "$0")" && pwd)
R=$(cd "$D" && { pwd -W 2>/dev/null || pwd; })
V="-v $R:/sim -v $R/root/data:/data -v $R/root/data_persist:/data_persist -v $R/root/etc-cmu-re:/etc/cmu-re -v rvp-target:/target"
BIN=/target/release/revised-daemon
case "$1" in
up)
  podman pod rm -f rvp-pod >/dev/null 2>&1; podman rm -f rvp-sim >/dev/null 2>&1
  podman pod create --name rvp-pod -p 127.0.0.1:2777:2777 -p 127.0.0.1:8750:8750 >/dev/null
  podman run -d --pod rvp-pod --name rvp-http -v "$R":/sim -v "$R/../catalog-dev":/catalog-dev:ro \
    -e ROOT=/sim/root -e CATALOG=/catalog-dev docker.io/library/node:22-alpine node /sim/serve.js >/dev/null
  podman run -d --pod rvp-pod --name rvp-sim $V rvp-build \
    $BIN --listen 0.0.0.0:2777 --ctl /tmp/rvp-ctl --catalog http://127.0.0.1:8750/catalog/ --dev-pair on --wlan eth0 >/dev/null
  sleep 2; podman logs rvp-http; podman logs rvp-sim 2>&1 | tail -3 ;;
down)
  podman pod rm -f rvp-pod >/dev/null 2>&1; echo "pod down" ;;
reset)
  podman pod rm -f rvp-pod >/dev/null 2>&1
  podman run --rm $V -v "$R/../catalog-dev":/catalog-dev:ro rvp-build sh -c "rm -rf /data_persist/cmu-re /data/integration/*; cd /sim && \
    $BIN --install-file /catalog-dev/pkgs/cmu-re-store-0.3.1.rvp --pubkey pubkey.minisign && \
    $BIN --install-file /catalog-dev/pkgs/t-hmi-hello-1.0.0.rvp --pubkey pubkey.minisign" ;;
js)
  code=$(printf '%s' "$2" | base64 -w0)
  n=$(podman logs rvp-sim 2>&1 | grep -c '"eval"')
  podman exec rvp-sim sh -c "echo 'js:eval(atob(\"$code\"))' > /tmp/rvp-ctl"
  i=0; while [ $i -lt 30 ]; do
    [ "$(podman logs rvp-sim 2>&1 | grep -c '"eval"')" -gt "$n" ] && break; sleep 0.2; i=$((i+1)); done
  podman logs rvp-sim 2>&1 | grep '"eval"' | tail -1 | sed 's/.*"action"://' ;;
ctl) podman exec rvp-sim sh -c "echo '$2' > /tmp/rvp-ctl" ;;
log) podman logs --tail "${2:-30}" rvp-sim 2>&1 ;;
opera) ("${OPERA:?set OPERA=<path to opera.exe> in local.env}" "http://127.0.0.1:8750/jci/gui/index.html" >/dev/null 2>&1 &) ;;
*) sed -n 2,11p "$0"; exit 1 ;;
esac
