- **`bun nv reference` runs its own `nvs`, not `target/debug/nvs`, so a fix just built can still
  fail there.** It takes `NVS_BIN` or the coverage workspace's binary that `bun nv verify` builds
  (`tools/nv/cmd/reference.ts`'s `nvsBinary`), so after a `cargo build` it reports the old help text.
  Rerun it as `NVS_BIN=<repo>/target/debug/nvs.exe bun nv reference`, with an absolute path, since a
  relative one is not found from the example's directory. `until: gone tools/nv/cmd/reference.ts:covwsNvs()`
