## Rules

### Presto and ES5

The HMI is the 2012 Opera (Presto) browser. `revised-sdk lint` enforces these rules on every `ui/**/*.js`, `ui/**/*.css` and `ui/**/*.html` file. `build` runs lint first and stops on errors. `build` does not print lint warnings (only errors): run `lint` to see them. Lint reads `style=""` attributes in HTML, but does not look inside `<style>` or `<script>` blocks: keep CSS and JavaScript in their own `.css` and `.js` files.

- JavaScript is ES5 only. Do not use `let`, `const`, arrow functions, `class`, template strings, spread or rest `...`, `for...of`, `async` or `await`. Use `var`, function expressions, string concatenation and indexed `for` loops. There is no `Promise` and no `fetch()`: use callbacks and `XMLHttpRequest`.
- CSS: no `display: flex`, `inline-flex`, `grid` or `inline-grid` (use `block`, `inline-block` or `float`), no `var(`, no `calc(`, no `clip-path`, no `display: table`, `table-cell` or `inline-table` (cells built after load do not repaint), no `-webkit-` properties. The vendor prefix is `-o-`: write it before the unprefixed form (`-o-linear-gradient(...)`, then `linear-gradient(...)`).
- Files must be valid UTF-8.
- Never reload or navigate the HMI page (`location.reload()`, `location.href = ...`). It reboots the whole unit about a minute later. To pick up changed code, use `push` or SafeReboot.

### Lint rule ids

Every finding has a `rule` id, a `level` (`error` or `warning`), and `file`, `line`, `col` (1-based) when it points at a place in a file. Errors give exit code 1. Warnings do not.

| Rule | Level | Meaning |
|---|---|---|
| `es5-syntax` | error | JavaScript that is not ES5, or a `.reload(` call (reloading the HMI page reboots the unit). The message names the replacement. |
| `css-presto` | error | CSS (or an inline `style=""`, or a line a `three-way` diff of a `.css` file adds) that Presto does not support. Positions in a diff are the diff's own lines. |
| `ws-literal-port` | warning | A hard-coded `ws://127.0.0.1:<port>` or `ws://localhost:<port>`. Take the endpoint from the HMI framework. |
| `hmi-handwritten` | warning | `ui/hmi.json` exists. `build` generates it and overwrites your copy. |
| `register-key` | error | No `RevisedHMI.register("<name>/...")` call in `ui/**/*.js`. The key must start with the `name` from `mod.yaml` plus `/`. |
| `encoding` | error | A file is not valid UTF-8. It is skipped. The finding points at the first bad byte. |
| `io` | error | A file cannot be read. |
| `mod-yaml` | error | `mod.yaml` is missing or does not parse. |
| `name` | error | `name` (or the `new` argument) is not `[a-z0-9][a-z0-9-]*`, at most 32 characters. |
| `version` | error | `version` uses characters outside `[0-9A-Za-z.+-]`, is longer than 64, starts with `.` or `-`, or contains `..`. |
| `target-cmu` | error | `targetCMU` is not `74.00.324A`, the only supported firmware. |
| `no-app` | error | `mod.yaml` has neither an `app:` block nor a `changes:` list. |
| `category` | error | `category` is not `Apps`, `Skins` or `System mods`. Without it, the category is `Apps` for a project with an app and `System mods` otherwise. |
| `change-live` | error | A change's `live` is not an absolute path on the unit. |
| `change-file` | error | A change's `file` is empty or absolute, contains a backslash, has an empty, `.` or `..` segment, or starts with the `ui` segment. |
| `change-reason` | error | A change's `reason` is empty. |
| `stock-path` | error | A change's `live` is not a regular file of the stock baseline list, is `/` or has a `..` segment. For a symlink, the message names the target to change instead. |
| `change-missing` | error | A change's `file` does not exist in the project. |
| `png-size` | error | A whole-file PNG has a different pixel size than the stock PNG it replaces. |
| `png-format` | error | A whole-file change of a `.png` is not a PNG file. |
| `diff-apply` | error | A `three-way` diff does not apply to the stock file in the local baseline directory. Line endings are compared as they are. |
| `no-baseline` | warning | No local baseline directory is recorded, so `three-way` diffs were not applied. Record one with `baseline check`. |
| `native-phase2` | error | `native:` is set. Native backends are not supported yet. |

Other commands report these ids:

| Rule | Level | Command | Meaning |
|---|---|---|---|
| `no-app` | error | `build` | `mod.yaml` has neither an `app:` block nor a `changes:` list (the same check as lint). |
| `rows` | error | `build` | `app.rows` is empty. |
| `stock-path` | error | `baseline png-size` | The path is not a PNG of the stock baseline list. |
| `stock-path` | error | `build` | A change's `live` has no mode in the stock baseline list. `build` never guesses a mode. |
| `entry` | error | `build` | `app.entry` is not a path under `ui/`. |
| `entry-missing` | error | `build` | `app.entry` does not exist. |
| `hmi-id` | error | `build` | An HMI id is longer than 32 characters: shorten `name`. |
| `pack` | error | `build` | The package or its manifest is invalid. The message names the cause. |
| `template` | error | `new` | Unknown template. |
| `exists` | error | `new` | The target directory exists and is not empty. |
| `baseline-missing` | error | `baseline check` | A listed stock file is missing from the directory. |
| `baseline-mismatch` | error | `baseline check` | A stock file's md5 differs from the published list. |
| `ntfs` | warning | `baseline check` | The directory is on a Windows drive; extract onto a POSIX file system. |
| `local-toml` | warning | `baseline check` | The baseline path could not be saved to `.revised/local.toml`. |
| `opera-sha256` | error | `opera install` | The download does not match the pinned sha256. It is deleted. |
| `devmode-off` | error | `push` | The unit is not in Developer mode. Nothing was installed. |
| `install-failed` | error | `push` | The daemon refused the package. The message has its reason. |
| `env` | error | any | Environment problem (no engine, unit unreachable, bad key). Exit code 2. |
| `io` | error | `new`, `build` | A file cannot be read or written. |

When a `preview` or `unit` action itself fails (exit 1), the rule id is the sub-command: `up`, `down`, `url`, `eval`, `shot`, `logs`, `key`.

### Jail rules

- A package owns only its own jail directory. Every app file lives under `ui/`; `build` generates `ui/hmi.json` and the manifest. Outside the jail, a package changes only the stock paths listed in `changes:`, each with a `reason`. For each path it carries replacement bytes (`whole-file`) or a diff (`three-way`), never stock file content. Changed files keep the stock mode and owner, and the HMI restarts after install.
- Package names and HMI ids match `[a-z0-9][a-z0-9-]*`, at most 32 characters. Lint enforces this (`name`).
- Lint and `build` check the stock paths of `changes:` against the published baseline list. The daemon checks the jail and stock paths again against the unit when it installs the package. The project must not claim any path in the store's own namespace. A package never contains a stock file or a modified full copy of one.
- Only `targetCMU: "74.00.324A"` is supported.
- `mod.yaml` is the single source of truth. Do not hand-write `ui/hmi.json` or a manifest.
- Package settings (the store's **Configuration** screen) go in the `config:` block of `mod.yaml`. Format and rules: `docs/config.md` in the SDK.

### Agent rules

- Search stock files only in the read-only baseline directory (the one `baseline check` recorded; mount it read-only if you search it from a container). Never run `grep -r`, `find` or any recursive search on the unit.
- Never remount the baseline read-write.
- Never copy stock files into the package or the repository.
- Run `push` only on a unit that is in Developer mode. `push` is refused otherwise. Ask the developer before every `push`.
- Never edit the unit's file system by hand (no `scp`, `cp`, `rm`, `chmod`, `sed` or remount over SSH). Changes reach the unit only as packages that the store installs and uninstalls.
- Never add anything above the `# cmu-re:tripwire` line in `/jci/scripts/start_normal_mode.sh`. That line starts the boot-loop guard, and code above it runs before the guard.
- File ownership is always numeric `uid:gid`, never a user name. Stock files are owned by `1000:1000`, and uid 1000 has no user name.
- Never write the `.up` password, or anything derived from it, into any file, log or message. Refer to it only as the environment variable `CMU_UP_PASS`.
- Do not edit `ui/hmi.json`, `dist/` or `.revised/`. They are generated or per-machine.
- Use `--json` and read the `ok`, `errors[].rule`, `errors[].file` and `errors[].line` fields. Do not parse the plain-text output.
- Fix the first error and run `lint` again. Do not suppress or work around a rule.
