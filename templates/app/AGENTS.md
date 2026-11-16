# AGENTS.md

Rules for coding agents that work on this Revised HMI app. Read every section before you change code.

## Rules

### Presto and ES5

The HMI is the 2012 Opera (Presto) browser. `revised-sdk lint` enforces these rules on every `ui/**/*.js`, `ui/**/*.css` and `ui/**/*.html` file. `build` runs lint first and stops on errors.

- JavaScript is ES5 only. Do not use `let`, `const`, arrow functions, `class`, template strings, spread or rest `...`, `for...of`, `async` or `await`. Use `var`, function expressions, string concatenation and indexed `for` loops. There is no `Promise`.
- CSS: no `display: flex`, `inline-flex`, `grid` or `inline-grid` (use `block`, `inline-block` or `float`), no `var(`, no `calc(`, no `clip-path`, no `-webkit-` and no `-o-` properties.
- Files must be valid UTF-8.

### Lint rule ids

Every finding has a `rule` id, a `level` (`error` or `warning`), and `file`, `line`, `col` (1-based) when it points at a place in a file. Errors give exit code 1. Warnings do not.

| Rule | Level | Meaning |
|---|---|---|
| `es5-syntax` | error | JavaScript that is not ES5. The message names the ES5 replacement. |
| `css-presto` | error | CSS (or an inline `style=""`) that Presto does not support. |
| `ws-literal-port` | warning | A hard-coded `ws://127.0.0.1:<port>` or `ws://localhost:<port>`. Take the endpoint from the HMI framework. |
| `hmi-handwritten` | warning | `ui/hmi.json` exists. `build` generates it and overwrites your copy. |
| `register-key` | error | No `RevisedHMI.register("<name>/...")` call in `ui/**/*.js`. The key must start with the `name` from `mod.yaml` plus `/`. |
| `encoding` | error | A file is not valid UTF-8. It is skipped. The finding points at the first bad byte. |
| `io` | error | A file cannot be read. |
| `mod-yaml` | error | `mod.yaml` is missing or does not parse. |
| `target-cmu` | error | `targetCMU` is not `74.00.324A`, the only supported firmware. |
| `changes-phase2` | error | `changes:` is not empty. System-mod edits are not supported yet. |
| `native-phase2` | error | `native:` is set. Native backends are not supported yet. |

`build` can also report `no-app`, `entry-missing` and `hmi-id` (the HMI id is `name`, at most 32 characters).

### Jail rules

- A package owns only its own jail directory. Every file in the package lives under `ui/`; `build` generates `ui/hmi.json` and the manifest.
- Package names and HMI ids match `[a-z0-9][a-z0-9-]*`, at most 32 characters.
- The project must not claim any path in the store's own namespace. Stock files are never part of a package.
- Only `targetCMU: "74.00.324A"` is supported.
- `mod.yaml` is the single source of truth. Do not hand-write `ui/hmi.json` or a manifest.

### Agent rules

- Search stock files only in the mounted read-only baseline (`/rootfs` in the SDK container). Never run `grep -r`, `find` or any recursive search on the unit.
- Never remount the baseline read-write.
- Never copy stock files into the package or the repository.
- Run `push` only on a unit that is in Developer mode. `push` is refused otherwise.
- Never write the `.up` password, or anything derived from it, into any file, log or message. Refer to it only as the environment variable `CMU_UP_PASS`.
- Do not edit `ui/hmi.json`, `dist/` or `.revised/`. They are generated or per-machine.
- Use `--json` and read the `ok`, `errors[].rule`, `errors[].file` and `errors[].line` fields. Do not parse the plain-text output.
- Fix the first error and run `lint` again. Do not suppress or work around a rule.

## API

`RevisedHMI` is a global that the HMI loads before your code. Version 1. ES5 only.

```js
RevisedHMI.apiVersion            // 1
RevisedHMI.register(key, {       // key = "<name>/<id>"; <name> must equal mod.yaml name
  open:  function (ctx) {},      // build your UI into ctx.root
  close: function () {}          // optional: stop timers and sockets
});
RevisedHMI.on(name, fn)          // read-only listeners:
                                 //   "screen" {uiaId, ctxtId}  on every context change
                                 //   "key"    {key, phase}     every key, cannot consume
                                 //   "ready"  {}               framework and registry loaded
RevisedHMI.log(msg)              // writes to the HMI log with a [rvp-sdk <key>] tag
```

`ctx`, the argument of `open`:

| Member | Meaning |
|---|---|
| `ctx.root` | The content element, under the header, inside the scroll area. Draw your UI here. |
| `ctx.setTitle(text)` | Sets the header title. |
| `ctx.onKey(fn)` | `fn({key, phase})`. Return `true` to consume the key. Active only while your app is open. |
| `ctx.close()` | Closes the app and returns to the stock screen that was showing before. |
| `ctx.assets` | Base URL of your `ui/` directory, for images and extra scripts. |
| `ctx.headerSlot` | An empty element at the right of the header, before the clock. It does not scroll. It is emptied when the app closes. |
| `ctx.scrollTo(y)` | Scrolls to `y` px from the top of `ctx.root`, clamped to the content height. |

There is no widget kit in v1: draw your own elements into `ctx.root`.

Behaviour:

- Only one app is open at a time.
- Back (the knob key, or the header back button) reaches `ctx.onKey` first as `{key: 8, phase: "down"}`. If your handler returns `true`, your app stays open. Otherwise `close()` runs and the app closes.
- Every call into your code is wrapped in try/catch. After the first throw, the entry or listener is disabled for that HMI session and the error is logged.
- The `main` script must call `RevisedHMI.register` when it loads. Otherwise the row stays and a log line is written.
- Knob and key codes: up 38, down 40, left 37, right 39, enter 13, back 8.

`ui/hmi.json` (generated by `build` from the `app:` block of `mod.yaml`):

```json
{"apiVersion": 1, "entries": [{"id": "demo", "section": "applications", "label": "demo", "main": "main.js", "listen": false}]}
```

`mod.yaml`:

```yaml
name: demo                   # package name and HMI id
version: 0.1.0
targetCMU: "74.00.324A"      # the only supported firmware
app:
  title: "demo"              # row label, at most 32 chars
  entry: ui/main.js
  icon: ui/icon.png
  rows: [apps]               # Apps and/or Entertainment
  background: false          # true: main.js is loaded at HMI boot (listeners only)
permissions: []
changes: []                  # must stay empty in this SDK version
```

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
| `setup` | Checks podman or docker and builds the SDK image. Run once. |
| `opera install --image` | Downloads Opera 12.10 into the `revised-sdk-opera` volume and checks its sha256. |
| `new <template> <name>` | Creates `<name>/` from a template (`app`). Also writes `AGENTS.md` and `CLAUDE.md`. |
| `lint` | Checks the project (see Rules). |
| `build` | Runs lint, then writes `dist/<name>-<version>.rvp` (unsigned). |
| `baseline check <dir>` | Compares an extracted stock rootfs with the published md5 list. |
| `preview up --accept-opera-license` | Starts the fake unit and a headless Opera 12.10. |
| `preview down` | Stops it. |
| `preview url` | Prints the page URL for your own Opera. |
| `preview eval '<js>'` | Runs JavaScript in the page and returns the result. |
| `preview shot <file.png>` | Screenshot, saved inside the project. |
| `preview key <name>` | Sends a key: `up down left right enter back home`. |
| `preview logs [n]` | Last `n` lines of the preview log (default 30). |
| `push [--unit cmu@<ip>] [--key <file>]` | Builds, copies the `.rvp` to the unit, installs it as unsigned. |
| `unit logs [n]` | Last `n` log lines that mention your package or an `[rvp-sdk` tag. |
| `unit shot <file.png>` | Screenshot of the unit's screen. |
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
- Preview binds `127.0.0.1` only: sim daemon on port 2777, HTTP on 8750. Running `preview up` twice reuses the running preview.
- The preview HMI is a small stub, not the stock HMI. It proves your app's code and layout, not the stock screens around it.

### Push and the unit

- `push` and `unit` use ssh port 36000, user `cmu`, with a key file. The key must be ECDSA or RSA: the unit's OpenSSH 5.9 has no ed25519, and the CLI refuses an ed25519 key before it connects. Make one with `ssh-keygen -t ecdsa -b 256 -f <keyfile>`.
- `--unit` and `--key` are saved to `.revised/local.toml` (git-ignored) so you pass them once.
- The unit must be in Developer mode with SSH enabled (key `.pub` on the USB stick). Otherwise `push` exits 1 with rule `devmode-off` and installs nothing.
- Host key: ssh runs with `StrictHostKeyChecking=accept-new`. The first connection trusts the unit's host key and stores it in your `known_hosts` (trust on first use). Later connections must match. If the unit was reflashed or replaced and ssh reports a changed host key, check that it is your unit, then remove the old entry with `ssh-keygen -R "[<ip>]:36000"` and connect again.
- Every `push` is an unsigned install and is logged by the unit as `unsigned-install`.
- `unit eval` needs the dev eval bridge on the unit. Without a reply within a few seconds the command fails and says so.
- Testing the CLI itself: `REVISED_SDK_WS_URL` points `push` at a loopback WebSocket. It accepts loopback addresses only. Developers do not need it.

## Stock baseline

The SDK needs the stock `74.00.324A` files to check your project against. This project never ships them. It publishes only a path and md5 list (`baseline/74.00.324A.md5`) and the steps below. You extract the baseline from the official update file (`.up`) that you already have.

The `.up` file is a password-protected ZIP. The password is not published here. It is well known in the CMU community, and it is stored in the stock firmware: look for a 16-character string in `libsecurity.so`. Set it in `CMU_UP_PASS` before you run the extraction.

**Why it is not published here.** The password belongs to the vendor's update format, not to this project. In some countries, publishing a key that opens a protected file can be treated as distributing a circumvention tool, even when the key is already public. A takedown would hit the whole repository, not one line. So this project ships only its own work: the extraction steps, file lists and hashes, never the key and never vendor files.

Extraction gives you a read-only stock baseline for diffs, lint and preview. It does not let you build firmware: an unpacked, modified and repacked `.up` is rejected by the unit's update check. Revised packages never modify or redistribute `.up` files.

### Extract

Use Linux, WSL or the SDK container. Do not extract on NTFS: it loses modes, owners and symlinks.

```sh
unzip -P "$CMU_UP_PASS" -p <ver>_update.up rootfs1upd/e0000000001.dat.gz | gunzip | gunzip > rootfs.tar
sudo mkdir -p stock/rootfs
sudo tar -xpf rootfs.tar --numeric-owner -C stock/rootfs
revised-sdk baseline check stock/rootfs
```

`baseline check` compares every listed file with the published md5 and reports `baseline-missing` and `baseline-mismatch` findings. It records the path in `.revised/local.toml`. A baseline that does not match is not genuine, unmodified `74.00.324A`: do not use it.

### Mount read-only

- SDK container: bind it as `-v ./stock/rootfs:/rootfs:ro`.
- As an image: `mkfs.ext4 -d stock/rootfs rootfs.img 3G`, then `mount -o loop,ro,norecovery rootfs.img <dir>`.
- Windows: `wsl --mount <vhd> --vhd --bare`, then `mount -o ro,norecovery <device> <dir>`.

Search stock files only there. Never search the unit.
