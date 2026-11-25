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

The full guide is `docs/guide.md`. It is assembled from `docs/agents/*.md`, the same text that goes into `AGENTS.md` in new projects. Edit the parts, then run `cli/t.sh env REVISED_DOCS_UPDATE=1 cargo test --test docs` to regenerate.

## License

Source-available under the [PolyForm Shield License 1.0.0](./LICENSE). You may use, modify and
freely share it for almost any purpose, including charging for a service you perform (such as
installing a free copy). You may not sell it or offer a competing product; the right to sell or
commercially distribute it is reserved to the project owner. This is source-available, not OSI
"open source".

Contributions are welcome under the [Individual CLA](./CLA.md); see [CONTRIBUTING.md](./CONTRIBUTING.md). Do not submit OEM (Mazda/JCI) binaries, keys or
PKI material: the repository is kept IP-clean.
