#!/bin/sh
# Remote driver for the bench CMU (DEV ONLY; needs the dev injector + revised-daemon --ctl).
#   u.sh js '<code>'   eval <code> in the HMI page, print {action:"ok|err", value}
#   u.sh sh '<cmd>'    run <cmd> on the unit (root)
#   u.sh shot <name>   screenshot -> shots-s4/<name>.png next to this script
#   u.sh ctl '<line>'  raw line to the FIFO (open | <keycode> | {json frame})
#   u.sh helpers       load rvp-helpers.js (window.__rvp) into the HMI page
# Runbook: revised-app-vault "Revised Store - App Deploy Runbook", section 5.
DIR=$(cd "$(dirname "$0")" && pwd)
[ -f "$DIR/local.env" ] && . "$DIR/local.env"   # CMU_KEY, CMU_HOST (git-ignored)
KEY=${CMU_KEY:?set CMU_KEY (unit ssh key) in local.env}
HOST=${CMU_HOST:?set CMU_HOST=cmu@<unit-ip> in local.env}
K="-o BatchMode=yes -i $KEY -p 36000 $HOST"
LOG=/tmp/revised-daemon.log
strip() { sed 's/^VPFS[0-9A-F]\{12,\}//'; }   # the unit's login banner
case "$1" in
js)
  # a page reload in the HMI reboots the whole unit ~1 min later (bench 2026-11-16)
  case "$2" in *.reload\(*) echo "refused: reloading the HMI page reboots the unit; use SafeReboot" >&2; exit 1 ;; esac
  # base64 so quotes/newlines survive ssh + FIFO; wait up to 6 s for the next "eval" log line
  code=$(printf '%s' "$2" | base64 -w0)
  ssh $K "n=\$(grep -c '\"eval\"' $LOG); echo 'js:eval(atob(\"$code\"))' > /tmp/rvp-ctl; i=0; while [ \$i -lt 30 ]; do [ \$(grep -c '\"eval\"' $LOG) -gt \$n ] && break; usleep 200000; i=\$((i+1)); done; grep '\"eval\"' $LOG | tail -1 | sed 's/.*\"action\"://'" 2>&1 | strip ;;
sh) ssh $K "$2" 2>&1 | strip ;;
helpers) sh "$0" js "$(cat "$DIR/rvp-helpers.js")" ;;
ctl) ssh $K "echo '$2' > /tmp/rvp-ctl" 2>&1 | strip ;;
shot)
  mkdir -p "$DIR/shots-s4"
  ssh $K 'cd /tmp && rm -f wayland-screenshot.png && XDG_RUNTIME_DIR=/tmp WAYLAND_DISPLAY=wayland-0 /usr/bin/screenshot >/dev/null 2>&1' 2>/dev/null
  scp -q -o BatchMode=yes -i "$KEY" -P 36000 $HOST:/tmp/wayland-screenshot.png "$DIR/shots-s4/$2.png" && echo "shot $DIR/shots-s4/$2.png" ;;
*) sed -n 2,8p "$0"; exit 1 ;;
esac
