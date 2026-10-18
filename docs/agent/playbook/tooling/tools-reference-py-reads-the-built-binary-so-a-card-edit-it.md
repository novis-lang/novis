- **`bun nv render --website` reads the *built* binary, so a card edit it has not been rebuilt for
  leaves `website/public/llms-full.txt` and `core.json` "already current".** The renderers ask
  `nvs meta` for the registry rather than parsing `registry.rs`, out of `NVS_BIN` or the newer of
  `target/debug` and `target/release`, and neither `cargo test` nor `bun nv verify` rebuilds those:
  verify builds the `covws` one. After fixing a `MethodDoc`, run `cargo build` and *then*
  `bun nv render --website`. `until: gone tools/nv/renderers/website-core.ts:nvs meta`
