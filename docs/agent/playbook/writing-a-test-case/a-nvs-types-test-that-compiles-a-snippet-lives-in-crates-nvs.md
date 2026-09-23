- **A `nvs-types` test that compiles a snippet lives in `crates/nvs-types/tests/`, never beside the
  registry tests in `src/`.** An inline `mod tests` in `core_lib.rs` asks the lowered registry its
  questions and never sees a diagnostic, so an anchor there is for the wrong host.
  `crates/nvs-types/tests/common/mod.rs`'s `check_in_method`/`check_src` are the harness and
  `crates/nvs-types/tests/core_members.rs` owns the options-bag rules; `cargo test -p nvs-types`
  runs both targets, so only the fixture decides. [until: reviewed 2026-09-06]
