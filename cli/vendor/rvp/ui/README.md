# Revised Store — test-app spike

Proves the end-to-end pattern for an on-HMI app: inject a row into the stock
**Apps** menu, open a full-screen overlay rendering every UI-kit control,
drive it with the **Commander knob**, and stream each interaction as JSON over
a WebSocket to a tiny **Rust** daemon that logs it.

```
Commander knob ──▶ overlay (controls.js) ──ws JSON──▶ ws-logger (Rust) ──▶ log
                   injected via /data/integration hook
```

Event shape (one per interaction):

```json
{ "control": "button", "action": "press", "value": "primary", "ts": 1699999999999 }
```

## Layout

| Path | What |
|------|------|
| `server/src/main.rs` | Rust WS logger (tungstenite 0.30, sync, 1 thread/conn, flushes each line) |
| `webapp/controls.js` | **the one renderer** — markup + styles + event shape + knob nav (ES5, Presto-safe) |
| `webapp/index.html` | offline demo host (served over http, shows a live event log) |
| `deploy/developer_mode_GuiConfig.js` | Tier-A injector: Apps-menu row + overlay, loaded by stock `index.html` |
| `deploy/ws-logger-arm` | ARM build of the daemon (ELF32, static musl, soft-float, ET_EXEC) |
| `deploy/run-on-unit.sh` / `revert-on-unit.sh` | on-unit deploy / reversal |
| `tools/ws-client.js` | manual WS test client (node v22+) |

## Commander knob mapping (intercept)

While the overlay is open, `controls.js` installs a **capturing** `keydown`
handler and swallows these so the HMI's own multicontroller does not also act.
Keycodes from `jci-ui-vault/05-framework-modules.md`:

| Input | keyCode | Action |
|-------|---------|--------|
| CW / right / down | 77 / 39 / 40 | focus next control |
| CCW / left / up | 78 / 37 / 38 | focus prev control |
| Enter (press) | 13 | activate focused control |
| Backspace (GoBack) | 8 | close overlay |

On close the handler is **torn down** (`ctl.teardown()`), releasing the knob
back to the HMI — do not skip this or the knob stays hijacked system-wide.

## Verified offline (this box)

- Renderer + all 6 control types emit correct events — live in the browser pane.
- Knob nav: ArrowDown×3 moved focus `Back → Warning`, Enter fired
  `button.press "warning"`, ArrowUp went back to `Secondary`, Backspace fired
  `back.press`; keys were swallowed (no page scroll).
- Rust logger logs events cleanly (6/6 via a WS client).

Offline demo is driven by two local containers + the native build; see
`git`-free notes in `tools/`. Not needed for the unit.

## Deploy to the GU

> **⚠️ These steps write to the unit.** Use a test unit. Loading the injector
> needs an HMI reload. Prefer restarting only the HMI over a full reboot: if a
> tripwire is installed, every reboot counts toward its boot-loop counter.
> SSH: `cmu@<unit-ip>`, port 36000, your unit key.

1. **Copy the bundle** (RW):
   ```sh
   scp -P 36000 -i <key> -r deploy cmu@<ip>:/tmp/rvp-spike
   ```
2. **Deploy** (RW): `ssh ... 'sh /tmp/rvp-spike/run-on-unit.sh'`
   — installs the 2 JS files to `/data/integration/` (0644, uid 1000) and starts
   the daemon from `/tmp` on `127.0.0.1:2777`.
3. **Reload the HMI** so `index.html` loads
   `../../data/integration/developer_mode_GuiConfig.js`.
4. **Watch**: `ssh ... 'tail -f /tmp/rvp-ws-logger.log'`
5. **Open**: Apps → **Revised Store**. Rotate = move, press = select,
   back = close. Each control logs a line.
6. **Revert** (RW): `ssh ... 'sh /tmp/rvp-spike/revert-on-unit.sh'` + HMI reload.

## Open checks on a real unit

Lines tagged `// VERIFY` in the injector, plus:

1. **Menu-row hook** — injector wraps `systemApp._buildApplicationsDataList`.
   Confirm the instance is reachable and the method name matches this build
   (the injector scans `window.systemApp` / `baseApp` / `framework.apps`).
2. **Row label** — `text1Id:"Revised Store"` assumes plain text; this build may
   want a localized string id.
3. **Row selection path** — assumes select → `framework.sendEventToMmui(uiaId,
   mmuiEvent)`; the injector intercepts `SelectRevisedStore` there.
4. **Presto WebSocket** — the framework itself uses `ws://127.0.0.1:2700/...`,
   so `ws://127.0.0.1:2777` should work the same; confirm the handshake.
5. **Knob keycodes** — confirm CW=77/CCW=78 on real hardware; adjust the map if
   this build differs.
6. **getrandom on kernel 3.0.35** — the WS *server* path uses SHA-1 only (no
   RNG), so the daemon should not touch getrandom; if it panics at startup,
   that's the cause — note it and we pin a `/dev/urandom` backend.

## Safety / reversibility

| Change | Persistent? | Reversal |
|--------|-------------|----------|
| `/data/integration/*.js` | yes (writable fs) | `rm` (revert script) |
| `/tmp/rvp-ws-logger` daemon | no (volatile) | `pkill` + gone on reboot |
| rootfs | **untouched** | — |
| reboot | not by scripts | — |
