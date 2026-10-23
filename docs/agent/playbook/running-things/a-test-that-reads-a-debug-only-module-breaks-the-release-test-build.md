- **A test that reads a `#[cfg(debug_assertions)]` module breaks every release build of its crate's
  tests, and no debug run shows it.** `nvs_host::counts` exists in debug builds only, and two
  `nvs-host` tests read it with no gate. So `cargo test --release --lib` failed with `cannot find
  counts in the crate root` after a 20-minute build. Put `#[cfg(debug_assertions)]` on every test
  that reads such a module, as `a_keep_alive_request_reads_and_wakes_within_its_count` in
  `crates/nvs-server/src/serve.rs` does. A release test run is needed to measure frame sizes,
  because a debug frame can be four times as large.
  `until: gone crates/nvs-host/src/lib.rs:#[cfg(debug_assertions)]`
