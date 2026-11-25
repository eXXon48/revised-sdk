# AGENTS.md

Instructions for coding agents (Claude Code, Cursor, Copilot, Codex and others) that help
someone build a mod or skin for a Mazda Connect CMU150 head unit with this SDK.

The code you write runs in a car. A mistake can leave the screen black or put the unit in
a reboot loop. Follow every rule below. If a step is not described here or in the files
this page links to, it does not exist yet: say so to the developer. Do not invent commands.

## Read first

| File | What it holds |
|---|---|
| [`docs/developer-sdk.md`](docs/developer-sdk.md) | Overview: what you can build, how a package reaches a car, Developer mode |
| [`docs/guide.md`](docs/guide.md) | Full reference: lint rules, the `RevisedHMI` API, every command, preview, push |
| [`docs/config.md`](docs/config.md) | Package settings (`config:` block) |
| `templates/app/AGENTS.md` | The rules a new project gets. Same text as `docs/guide.md`. |

Inside a project made with `revised-sdk new`, read that project's `AGENTS.md` before you
change code.

## Set up the environment

1. Install podman or docker.
2. Build the CLI: `cargo build --release` in `cli/` (Rust 1.99.1, crates are vendored).
   Put the `revised-sdk` binary on the `PATH`.
3. Run `revised-sdk setup --store <cmu-re-store checkout>` once. It builds the SDK image
   from `container/Containerfile`, with the store checkout as a second build context.
   `--store` is required for now because the store repo is not public yet. Add `--sdk <this repo>` when the binary does not run from
   inside this checkout. `--dry-run` prints the build command and builds nothing.
4. For the preview, run `revised-sdk opera install --image`. Opera 12.10 comes under its
   own license.
5. Make a project: `revised-sdk new app <name>`. Names match `[a-z0-9][a-z0-9-]*`, at most
   32 characters.

## Package format

- `mod.yaml` is the only file you write by hand. `build` generates `ui/hmi.json` and the
  package manifest from it. Do not edit `ui/hmi.json`, `dist/` or `.revised/`.
- Main fields: `name`, `version`, `targetCMU` (only `"74.00.324A"`), `app:` (`title`,
  `entry`, `icon`, `rows`, `background`), `permissions`, `config`, `changes`.
- Every file of the package lives under `ui/`. On the unit it goes to the package
  directory `/data_persist/cmu-re/pkg/<name>/`. A package never contains stock files.
- `revised-sdk build` writes `dist/<name>-<version>.rvp`, unsigned.
- In the manifest each file entry has `live` (target path), `payload`, `mode`, `owner`,
  `type` (`new`, `modify` or `link`) and `apply` (`whole-file`, `xml-node` or `3way`).
  `owner` is always numeric `uid:gid`. Stock files are owned by `1000:1000` and there is
  no user name for uid 1000, so never use a name.
- Revert: the store daemon records what it changed at install. Uninstall puts the unit
  back exactly as it was before the install. You do not write revert steps.
- Not supported yet: edits to stock system files (`changes:` must stay empty) and native
  backends (`native:`). `lint` refuses both.

## Test

1. `revised-sdk lint`, then `revised-sdk build`. Fix the first error, run again. Use
   `--json` and read `ok` and `errors[]`.
2. `revised-sdk preview up --accept-opera-license` runs your package against a simulated
   HMI on the PC. The flag accepts the Opera 12.10 EULA: ask the developer before you pass
   it. Then use `preview eval`, `preview shot`, `preview key` and `preview logs`.
3. A real unit only through the store in Developer mode: `revised-sdk push --unit
   cmu@<ip> --key <ecdsa-key-file>`. It installs the package as unsigned. Ask the
   developer before every push. `unit logs`, `unit shot` and `unit eval` read the unit.
4. To remove a package from the unit, uninstall it in the store. Leaving Developer mode
   removes every unsigned package.

`cmu-re-store` has its own local HMI simulator (`local-sim/u.sh`). It is for work on the
store itself. Package authors use `revised-sdk preview`.

## Hard rules

- The HMI is the 2012 Opera (Presto) browser. JavaScript is ES5 only: no `let`, `const`,
  arrow functions, `class`, template strings, `...`, `for...of`, `async`, `await` or
  `Promise`. CSS: no `flex`, `grid`, `display: table-cell`, `var()`, `calc()`,
  `clip-path`, no `-webkit-` and no `-o-` properties. `lint` checks this. Do not work
  around a lint rule.
- Never call `location.reload()` and never navigate the HMI page. It reboots the whole
  unit about one minute later. To load changed code, `push` again.
- Never edit the unit's file system by hand: no `scp`, `sed`, `cp`, `rm`, `chmod` or
  remount over SSH. Changes reach the unit only as packages that the store installs and
  uninstalls.
- Never add anything above the `# cmu-re:tripwire` line in
  `/jci/scripts/start_normal_mode.sh`. That line runs the boot-loop guard. Code above it
  runs before the guard and can lock the unit in a reboot loop.
- File ownership is always numeric `uid:gid`. Stock files are `1000:1000`.
- Never run `grep -r`, `find` or any recursive search on the unit. Search stock files only
  in the read-only baseline on the PC (see "Stock baseline" in `docs/guide.md`).
- Never write the `.up` password into a file, log or message. It lives only in
  `CMU_UP_PASS`. Never copy stock files into a package or a repository.
- Never reboot the unit, push, or run anything that writes to it without the developer's
  clear yes.

## Work on the SDK itself

- `cli/t.sh cargo test` runs the CLI tests in the `rvp-build` image. That image comes
  from the store repo's `Containerfile`. There is no public recipe for it yet, so this
  step is only for maintainers who already have the image. Package authors do not need it.
- `docs/guide.md` and `templates/app/AGENTS.md` are generated from `docs/agents/*.md`.
  Edit the parts, then run `cli/t.sh env REVISED_DOCS_UPDATE=1 cargo test --test docs`.
