- **`target/debug/nvs.exe` is not what the pipeline builds, so a probe run on it can test a binary
  days old or none at all.** The sweep, `bun nv verify`, `bun nv try` and `bun nv reference` build and
  run `target/covws/<host triple>/debug/nvs.exe` (`tools/nv/lib/covws.ts`), and only a `cargo build`
  by hand writes `target/debug`. Where a bullet or a habit names `target/debug/nvs.exe`, run the covws
  binary, or `cargo build` first.
  [until: gone tools/nv/lib/covws.ts:covwsNvs]
