- **A path a test passes to `cmd /C` is rebuilt with `\` first.** `CARGO_TARGET_TMPDIR` can arrive
  as `E:/mwl/target/tmp` under `bun nv verify`, and `cmd` reads `/mwl` as a switch, so `mklink /J`
  failed with "invalid switch" while a plain `cargo test` passed. Collect the path's `components()`
  into a `PathBuf` before `.arg`, as `point` in `crates/nvs-cli/tests/live_edit.rs` does.
  `until: gone crates/nvs-cli/tests/live_edit.rs:let native = |path: &Path|`
