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
