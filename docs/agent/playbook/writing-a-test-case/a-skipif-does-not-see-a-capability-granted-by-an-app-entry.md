- **A `--SKIPIF--` does not see a capability granted by an `[[app]] entry = "case.nvs"` block.**
  `crates/nvs-test/src/run.rs` runs it as `skipif.nvs` before it writes `case.nvs`, so its config
  fails with E0605, it prints nothing, and the runner runs the case: green where the service is up,
  red in CI. Grant what the SKIPIF needs at tree level, as `[capabilities.cache] shared = true` in
  `tests/conformance/core/session-get-answers-null-for-a-sealed-value.nvst` does.
  [until: test skipif_that_exits_nonzero_fails_the_case]
