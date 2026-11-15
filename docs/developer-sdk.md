# Revised Developer SDK

Build packages for the Mazda CMU150 (firmware `74.00.324A`) that install through the
Revised Store. This page is the overview: what you can build, how a package reaches a car,
and how to test on a real unit. The command and API reference is [`guide.md`](guide.md).

## What you can build today

| Kind | What it is | How |
|---|---|---|
| HMI app | A screen of your own, opened from a row in Apps or Entertainment | `revised-sdk new app <name>`, then the `app:` block in `mod.yaml` |
| Background mod | Code that loads with the HMI and listens to screens and keys, with no menu row | `app.rows: []` and `app.background: true` |
| Package settings | A Configuration screen for your package, values readable by your code | `config:` block, see [`config.md`](config.md) |
| On-demand service | A process of your own that your app starts and stops | `RevisedHMI.service()`, see the API in [`guide.md`](guide.md) |

Not supported yet: edits to stock system files (`changes:` must stay empty) and native
backends (`native:`). `lint` refuses both.

## The workflow

1. `revised-sdk setup --store <cmu-re-store checkout>` once, then `revised-sdk new app <name>`.
2. Write `ui/main.js` and `ui/style.css` for the 2012 Opera (Presto) engine: ES5 JavaScript,
   no flexbox, grid, `var()` or `calc()`. `revised-sdk lint` checks every rule.
3. `revised-sdk build` writes `dist/<name>-<version>.rvp`.
4. `revised-sdk preview up` runs the package against a simulated HMI on your PC.
5. `revised-sdk push` installs it on a real unit in Developer mode (below).

Every command takes `--json`, so a coding agent can drive the same loop. New projects get an
`AGENTS.md` with the same rules as [`guide.md`](guide.md).

## How a package reaches a car

- **Unsigned.** `build` produces an unsigned package. It installs only on a unit in Developer
  mode, through `revised-sdk push` or from a USB stick, and the unit logs each such install.
- **Signed.** Packages listed in the store catalog are signed by CMU-Revised after review. The
  SDK never signs and never needs a key for it.

## Where a package lives on the unit

- Your files go to your package directory, `/data_persist/cmu-re/pkg/<name>/`. It survives a
  factory reset and is removed when the package is uninstalled.
- A package never contains stock files and never claims a path that belongs to the store.
- Uninstall puts the unit back exactly as it was before the install.

## Developer mode

Turn it on in the store on the unit to test unsigned packages.

- **Enable.** Account → `Enable Developer mode` (the car must be linked and online). A `DEV`
  badge appears in the header and a `Developer` section appears in the menu.
- **SSH.** Key-only, port 36000, user `cmu`. Make a key with
  `ssh-keygen -t ecdsa -b 256 -f cmu_key` (the unit's OpenSSH 5.9 has no ed25519; RSA also
  works). Copy `cmu_key.pub` to the root of a USB stick as `.cmu-re_ssh-publickey`, insert the
  stick, then press `Enable SSH` and restart. The screen shows the `ssh` command with the
  unit's current Wi-Fi address. `Disable SSH` removes the key. To change the key, disable,
  then enable again with the new file on the stick.
- **Unsigned installs** work only while Developer mode is on. Each USB install shows its own
  consent screen with the permissions and the stock files it touches.
- **Leave Developer mode** removes SSH and every unsigned package, and turns the recovery
  switches back on.
- **Brick support.** A unit that was in Developer mode is not supported if it bricks. Leaving
  does not restore support. Only a full firmware reflash (`.up`) followed by a clean store
  install does. The Account screen shows `Brick support: OFF` until then.

## Stock baseline

Checking a package against stock files needs the stock `74.00.324A` file system. This
project never ships Mazda files: it publishes only a path and md5 list
(`baseline/74.00.324A.md5`) and the steps to extract the file system from the official
update file you already have. The update file's password is not published; set it in
`CMU_UP_PASS`. Steps and the reasons: "Stock baseline" in [`guide.md`](guide.md).
Plain HMI apps do not need the baseline.

## Documents

| File | Contents |
|---|---|
| [`guide.md`](guide.md) | Quick start, lint rules, the `RevisedHMI` API, commands, preview, push, baseline |
| [`config.md`](config.md) | Package settings: fields, output, templates |
| `agents/*.md` | The parts `guide.md` and `AGENTS.md` are built from |
| `../baseline/` | The published stock path and md5 list |
