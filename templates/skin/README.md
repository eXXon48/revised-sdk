# {{name}}

A skin for the Mazda CMU, built with `revised-sdk`. It changes two stock files:

| Stock file | Project file | How |
|---|---|---|
| `/jci/gui/common/images/background.png` | `skin/background.png` | whole file, same pixel size as stock (800x480) |
| `/jci/gui/common/controls/List2/css/List2Ctrl.css` | `skin/List2Ctrl.css.diff` | three-way diff of the list item text colour |

- `src/background.svg` is the source of the background. Edit it, then run `sh tools/render.sh` in the SDK image to write `skin/background.png`.
- The diff holds only the changed line and one line of stock context on each side. Never put a whole stock file, or a modified copy of one, into the project.
- `mod.yaml` lists the changes, each with a `reason`.

```
revised-sdk lint       # png-size, diff-apply and CSS checks
revised-sdk build      # writes dist/{{name}}-<version>.rvp
```

See "Skins" in the SDK guide for the full workflow.
