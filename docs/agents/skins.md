## Skins

A skin is a package with `category: Skins` and no app. It changes stock HMI files through `changes:` in `mod.yaml`. Start from the template: `revised-sdk new skin myskin`. It replaces the screen background (`/jci/gui/common/images/background.png`, a whole-file PNG) and changes the list item text colour in `/jci/gui/common/controls/List2/css/List2Ctrl.css` (a three-way diff).

### Ownership and switching

A package owns the stock files it replaces whole-file. Installing a skin that replaces a file another installed package also replaces whole-file is a switch: the store uninstalls the other package and installs the new one in one step, and its confirm pane shows `Replaces: <title>` first. A whole-file change and a `three-way` diff on the same file from different packages conflict: the install is refused and names the other package. `three-way` diffs from different packages stack when they apply. When the last package that changes a file is uninstalled, the stock file is restored.

### PNG size

A whole-file PNG must have the exact pixel size of the stock PNG it replaces (`png-size`). Get the size from the published list:

```sh
revised-sdk baseline png-size /jci/gui/common/images/background.png     # prints: 800 480
```

### CSS is a diff, never a full file

Change a stock CSS file with a `three-way` unified diff, never with a whole-file copy. The diff carries only your changed lines and the few context lines the unified format needs to place them. Context lines are short fragments of the stock file, never the whole file. A full copy of a stock file is stock content, and a package never carries stock content. A diff also applies on top of the stock file the unit has, so it keeps working next to other changes. Lint checks the lines the diff adds against the CSS rules and, with a local baseline, applies the diff to the stock file (`diff-apply`).

### Asset workflow

1. Measure: `revised-sdk baseline png-size <live>` for each PNG you replace.
2. Draw: keep the art as SVG in `src/`, at that size (`width`, `height` and `viewBox`).
3. Render: `sh tools/render.sh` in the SDK image (it has `rsvg-convert`) writes the PNG at the stock size.
4. Check: `revised-sdk lint`. With a baseline recorded by `baseline check`, lint also applies every diff.
5. Build: `revised-sdk build`.
6. Look: `revised-sdk preview up --accept-opera-license --stock`, then `preview ctx` to open a screen and `preview shot` to save it.

### Preview against the stock GUI

- `preview up --stock [dir]` runs the stock GUI from the baseline directory (default: the one `baseline check` recorded) with stub sockets, then builds the project and installs it, so your changes show on the stock screens. The baseline is mounted read-only. The copy of the GUI lives in the container, and every `up` starts from a fresh copy.
- `preview ctx <uiaId> <ctxtId>` opens a stock screen, for example `preview ctx system Entertainment` or `preview ctx fmradio StationList`. Ids are letters, digits or `_`.
- `preview up --vnc [--vnc-port n]` adds a live view on `vnc://127.0.0.1:5900`. The VNC server has no password and is published on loopback only, but other containers on the same engine network can reach it.
