## Loop

Work in this loop: `new`, `lint`, `build`, `preview up`, `eval` and `shot`, then `push`. All commands accept `--json`.

### Output contract

- Plain text by default. With `--json`, exactly one JSON object on stdout; logs go to stderr.
- Shape: `{"ok": bool, "command": "<name>", "data": {...}, "errors": [{"file", "line", "col", "level", "rule", "msg"}]}`.
- Exit code `0`: ok. `1`: a check failed or the action was refused. `2`: usage or environment error (for example no container engine; the message names the fix).
- No command asks a question. Anything that would ask takes a flag.
- Global flags: `--json`, `--engine podman|docker`, `-C <project dir>`.

### Commands

| Command | Does |
|---|---|
| `setup --store <cmu-re-store checkout> [--sdk <revised-sdk checkout>] [--dry-run]` | Builds the SDK image with podman or docker. Run once. `--store` is required (phase 1: the store repo is private). `--sdk` is needed only when the binary does not run from inside a revised-sdk checkout. `--dry-run` prints the engine arguments and builds nothing. |
| `opera install --image` | Downloads Opera 12.10 into the `revised-sdk-opera` volume and checks its sha256. |
| `new <template> <name>` | Creates `<name>/` from a template: `app` (also writes `AGENTS.md` and `CLAUDE.md`) or `skin` (see Skins). |
| `lint` | Checks the project (see Rules). |
| `build` | Runs lint, then writes `dist/<name>-<version>.rvp` (unsigned). |
| `baseline check <dir>` | Compares an extracted stock rootfs with the published md5 list. |
| `baseline png-size <live>` | Prints the pixel size of a stock PNG as `W H` (`--json`: `width`, `height`). |
| `preview up --accept-opera-license [--stock [dir]] [--vnc [--vnc-port n]]` | Starts the fake unit and a headless Opera 12.10. `--stock` runs the stock GUI from the baseline, `--vnc` adds a live view (see Skins). |
| `preview down` | Stops it. |
| `preview url` | Prints the page URL for your own Opera. |
| `preview eval '<js>'` | Runs JavaScript in the page and returns the result. |
| `preview shot <file.png>` | Screenshot, saved inside the project. |
| `preview key <name>` | Sends a key: `up down left right enter back home`. |
| `preview logs [n]` | Last `n` lines of the preview log (default 30). |
| `preview ctx <uiaId> <ctxtId>` | Opens a stock screen (only with `--stock`). |
| `push [--unit cmu@<ip>] [--key <file>]` | Builds, copies the `.rvp` to the unit, installs it as unsigned. |
| `unit logs [n]` | Last `n` log lines that mention your package or an `[rvp-sdk` tag. |
| `unit shot <file.png>` | Screenshot of the unit's screen. The path must not start with `-` or contain `:`. |
| `unit eval '<js>'` | Runs JavaScript in the unit's HMI, through the dev eval bridge. |

### Examples with `--json`

```sh
revised-sdk --json new app demo
{"ok":true,"command":"new","data":{"path":"demo"},"errors":[]}

revised-sdk --json lint        # clean
{"ok":true,"command":"lint","data":{},"errors":[]}

revised-sdk --json lint        # exit 1
{"ok":false,"command":"lint","data":{},"errors":[{"file":"ui/main.js","line":23,"col":12,"level":"error","rule":"es5-syntax","msg":"arrow function is not ES5: use `function () {}`"}]}

revised-sdk --json build
{"ok":true,"command":"build","data":{"path":"dist/demo-0.1.0.rvp"},"errors":[]}

revised-sdk --json baseline check stock/rootfs     # exit 1 when files differ or are missing
{"ok":false,"command":"baseline check","data":{"checked":8464,"mismatch":["<path>"],"missing":["<path>"]},"errors":[...]}

revised-sdk --json preview up --accept-opera-license
{"ok":true,"command":"preview up","data":{"url":"http://127.0.0.1:8750/jci/gui/index.html"},"errors":[]}

revised-sdk --json preview eval 'document.title'
{"ok":true,"command":"preview eval","data":{"result":"..."},"errors":[]}

revised-sdk --json preview shot shots/home.png
{"ok":true,"command":"preview shot","data":{"path":"..."},"errors":[]}

revised-sdk --json preview key enter
{"ok":true,"command":"preview key","data":{"key":"enter","code":13},"errors":[]}

revised-sdk --json preview logs 20
{"ok":true,"command":"preview logs","data":{"lines":["..."]},"errors":[]}

revised-sdk --json push --unit cmu@<ip> --key <ecdsa-key-file>
{"ok":true,"command":"push","data":{"name":"demo","version":"0.1.0","restart":false},"errors":[]}

revised-sdk --json unit logs 20     ->  data: {"lines": [...]}
revised-sdk --json unit shot unit.png  ->  data: {"path": "unit.png"}
revised-sdk --json unit eval '1+1'  ->  data: {"result": ...}
```

In `baseline check`, `missing` and `mismatch` list paths. In `push`, `restart` is the daemon's answer.

### Preview

- `preview up` needs `--accept-opera-license`. Opera 12.10 is not part of this project and comes under its own license (EULA). By passing the flag you, the developer, accept that EULA on this machine. Without the flag the command exits 2 and prints how to read the license. An agent must not pass the flag on its own: ask the developer first.
- Install Opera once with `opera install --image`.
- Preview binds `127.0.0.1` only: sim daemon on port 2777, HTTP on 8750. Running `preview up` again replaces the running preview with a fresh one (there is only ever one preview container).
- Without `--stock`, the preview HMI is a small stub, not the stock HMI. It proves your app's code and layout, not the stock screens around it. With `--stock` it runs the stock GUI from your baseline (see Skins).

### Push and the unit

- `push` and `unit` use ssh port 36000, user `cmu`, with a key file. The key must be ECDSA or RSA: the unit's OpenSSH 5.9 has no ed25519, and the CLI refuses an ed25519 key before it connects. Make one with `ssh-keygen -t ecdsa -b 256 -f <keyfile>`.
- Only `push` saves `--unit` and `--key` to `.revised/local.toml` (git-ignored), so you pass them once. `unit logs|shot|eval` read the saved values but do not save new ones.
- The unit must be in Developer mode with SSH enabled (key `.pub` on the USB stick). Otherwise `push` exits 1 with rule `devmode-off` and installs nothing.
- Host key: ssh runs with `StrictHostKeyChecking=accept-new`. The first connection trusts the unit's host key and stores it in your `known_hosts` (trust on first use). Later connections must match. If the unit was reflashed or replaced and ssh reports a changed host key, check that it is your unit, then remove the old entry with `ssh-keygen -R "[<ip>]:36000"` and connect again.
- Every `push` is an unsigned install and is logged by the unit as `unsigned-install`.
- `unit eval` needs the dev eval bridge on the unit. Without a reply within a few seconds the command fails and says so.
- Testing the CLI itself: `REVISED_SDK_WS_URL` points `push` at a loopback WebSocket. It accepts loopback addresses only. Developers do not need it.
