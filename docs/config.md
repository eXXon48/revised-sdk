# Package settings (`config`)

A package can declare settings. The store shows them on the package page as a
**Configuration** button (installed packages only). The user picks values with the knob or
touch. The store writes the values into files of your package from templates you ship.

> **SDK status:** `mod.yaml` has no `config:` key yet, so `revised build` does not emit this
> block. Until it does, add `config` to the package `manifest.json` by hand and pack it with
> `rvp-pack`.

There is no free-text field. Every value comes from a list or a numeric range you declare, so a
value can never break the syntax of the file it lands in. The store does no escaping.

## How it works

1. Ship one or more **templates** in your package, for example `ui/settings.js.tpl`.
2. Declare **fields** (the settings) and **outputs** (template -> rendered file) in the
   manifest's `config` block.
3. On install the store renders every output with the **default** values. The rendered file
   exists before your app first runs.
4. When the user changes values and presses **Apply**, the store renders the outputs again and
   applies them as `apply` says.
5. On update the store keeps the user's values that are still valid under the new schema.
   It uses the defaults for new fields and for values that are no longer valid. It drops keys
   you removed.
6. On uninstall the store deletes the stored values with the package.

Your app reads the rendered file like any other file of the package. It never talks to the store
to get its settings.

## Example

Template `ui/settings.js.tpl`, loaded by your app with `<script src="settings.js">`:

```js
var SETTINGS = {
  accent: "{{ accent }}",
  fontSize: {{ size }},
  rows: {{ rows }},
  showClock: {{ clock }}
};
```

The `config` block in `manifest.json`:

```json
"config": {
  "fields": [
    { "key": "accent", "type": "select", "label": "Accent colour", "default": "#FF2A30",
      "options": [
        { "label": "Red",   "value": "#FF2A30" },
        { "label": "Green", "value": "#2ECC71" },
        { "label": "Blue",  "value": "#2E86FF" }
      ] },
    { "key": "size",  "type": "range",  "label": "Font size",  "default": 16, "min": 12, "max": 24, "step": 2 },
    { "key": "rows",  "type": "number", "label": "Rows",       "default": 3,  "min": 1,  "max": 9 },
    { "key": "clock", "type": "switch", "label": "Show clock", "default": true }
  ],
  "outputs": [
    { "template": "ui/settings.js.tpl", "path": "ui/settings.js" }
  ],
  "apply": "none"
}
```

With the defaults, the store writes `ui/settings.js`:

```js
var SETTINGS = {
  accent: "#FF2A30",
  fontSize: 16,
  rows: 3,
  showClock: true
};
```

The test package `t-skin-accent` in `cmu-re-store/daemon/testpkgs/` uses every field type.
It renders a CSS file and an INI file.

## Format

### `config`

| Key | Type | Required | Meaning |
|-----|------|----------|---------|
| `fields` | array of field | yes, at least 1 | The settings, in the order the store shows them. |
| `outputs` | array of output | yes | Templates to render. |
| `apply` | string | no, default `"none"` | What happens after the store writes new values. See below. |

### Field: common keys

| Key | Type | Meaning |
|-----|------|---------|
| `key` | string | Name used in templates as `{{ key }}`. Must match `[a-z][a-z0-9_]*` and be unique. |
| `type` | string | `select`, `range`, `number` or `switch`. |
| `label` | string | Text shown in the store. Must not be blank. |
| `default` | depends on type | Value on first install. Must be a valid value for the field. |

### `select`

A list. The store opens it as an option list; the user picks one row.

| Key | Type | Meaning |
|-----|------|---------|
| `options` | array of `{ "label": string, "value": string }` | At least 2. Labels not blank, values unique. |
| `default` | string | One of the option `value`s. |

Template output: the option `value`, as is.

### `range`

A slider with steps.

| Key | Type | Meaning |
|-----|------|---------|
| `min`, `max` | integer | `min < max`. |
| `step` | integer | `step > 0`, and `max - min` must be a multiple of `step`. |
| `default` | integer | In range and on a step (`(default - min) % step == 0`). |

Template output: the integer in decimal.

### `number`

A `[-] value [+]` stepper with step 1.

| Key | Type | Meaning |
|-----|------|---------|
| `min`, `max` | integer | `min <= max`. |
| `default` | integer | `min <= default <= max`. |

Template output: the integer in decimal.

### `switch`

An on/off toggle.

| Key | Type | Meaning |
|-----|------|---------|
| `default` | boolean | `true` = on. |
| `on` | string | Optional, default `"true"`. Written when the switch is on. |
| `off` | string | Optional, default `"false"`. Written when the switch is off. |

Template output: the `on` or `off` string. Example: `"on": "block", "off": "none"` for a CSS
`display` value.

### Output

| Key | Meaning |
|-----|---------|
| `template` | The `live` path of a file in your package. It must be a jailed (relative) `type: "new"` file. |
| `path` | Where the rendered file goes, relative to your package directory. |

Rules for `path`:

- It is relative and has no `..`, `.` or empty parts.
- It is not a file of the package, and not listed twice.
- It is not a directory of a package file or of another output, and not under one.
- `<path>.cfgtmp` is not taken. The store renders into that temp file first, then renames it.

A path that is a symlink, or has a symlink in it, is refused when the store renders.

### `apply`

| Value | After Apply |
|-------|-------------|
| `none` | Files are written. Your app picks up the change the next time it reads the file. |
| `restart-service` | The package's services stop, the files are written, the services start again. |
| `reboot` | Files are written. The store asks the user to restart the head unit. |

## Templates

- `{{ key }}` is replaced by the field's output. Spaces inside the braces are allowed.
- Every placeholder must name a declared field. An unknown key, or `{{` without `}}`, fails the
  render.
- The template is copied as it is everywhere else. The store does not know the file format.
- You write the quotes the format needs. In the example above, the `select` value goes inside
  `"..."` and the numbers and the boolean do not.
- Choose option values and `on`/`off` strings that are safe in your file format. A value that
  contains `"` would break a JSON or JS string. The store does not check this.

## Where values live on the unit

- Rendered files: in your package directory, `/data_persist/cmu-re/pkg/<name>/<path>`.
- Stored values: `/data_persist/cmu-re/store/config/<name>.json`. Do not write this file
  from your package. Only the store writes it.

## Errors

- The manifest is checked when the package is built and installed. A bad `config` block
  refuses the package with a message that starts with `config:`.
- If the package installs but a render fails, the install still succeeds. The store shows
  "installed, but config of `<name>` not written: ...". The values are saved, so
  **Configuration -> Apply** can retry.
- If Apply fails, the store shows the error on the Configuration screen and keeps the old files.
