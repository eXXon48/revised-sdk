# revised-sdk

SDK for CMU-Revised package authors. Planned pieces: the `revised-sdk` CLI, a build container, and an Opera 12.10 preview. `docs/developer-sdk.md` is the overview, `docs/guide.md` the reference. `baseline/` is the published stock manifest for firmware 74.00.324A: paths and hashes only, with no Mazda files.

## Install and quick start

Build the CLI with `cargo build --release` in `cli/` (Rust 1.99.1; crates are vendored), put `revised-sdk` on your `PATH`, then:

```sh
revised-sdk setup --store <cmu-re-store checkout>  # once: builds the SDK image (needs podman or docker; --sdk, --dry-run)
revised-sdk new app mydash && cd mydash
revised-sdk lint && revised-sdk build              # dist/mydash-0.1.0.rvp
revised-sdk preview up --accept-opera-license      # accepts the Opera 12.10 EULA; see docs/guide.md
```

The full guide is `docs/guide.md`. It is assembled from `docs/agents/*.md`, the same text that goes into `AGENTS.md` in new projects. Edit the parts, then run `REVISED_DOCS_UPDATE=1 cli/t.sh cargo test --test docs` to regenerate.
