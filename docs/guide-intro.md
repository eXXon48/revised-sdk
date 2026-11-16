# Revised SDK guide

`revised-sdk` builds, checks, previews and installs HMI apps for the Mazda CMU150 (firmware `74.00.324A`) through CMU-Revised. It runs on your PC. Its output is an unsigned `.rvp` package.

This guide and the `AGENTS.md` file in every new project come from the same source, so you and your coding agent read the same rules. Point your agent at `AGENTS.md` (`CLAUDE.md` in the project says only `Read AGENTS.md.`).

## Quick start

1. Install podman or docker, then run `revised-sdk setup` once. It builds the SDK image.
2. Install Opera 12.10 for the preview: `revised-sdk opera install --image`. Opera is not part of this project and comes under its own license.
3. Make a project: `revised-sdk new app mydash`, then `cd mydash`.
4. Edit `ui/main.js` and `ui/style.css`. Run `revised-sdk lint` until it is clean.
5. `revised-sdk build` writes `dist/mydash-0.1.0.rvp`.
6. `revised-sdk preview up --accept-opera-license` starts a fake unit and prints a URL. Passing `--accept-opera-license` means you accept the Opera 12.10 EULA on this machine. Open the URL in your own Opera 12.10, or let your agent use `preview eval`, `shot` and `key`.
7. For a real unit: turn on Developer mode on the unit, then `revised-sdk push --unit cmu@<ip> --key <ecdsa-key-file>`.

Your project has a `mod.yaml` (package description) and a `ui/` directory (your app). The stock baseline (below) is optional for plain HMI apps. It is used by `baseline check` and, later, by deeper checks.

The sections below are the shared reference.
