- **A test helper that names a file in `CARGO_TARGET_TMPDIR` after its *input* races the other tests
  that ask for the same input.** Tests run in parallel, so a reader catching another thread's
  `fs::write` half-done sees a truncated document: intermittently, and never under `cargo test --
  <one test name>`. Run the *whole* binary on the stashed tree; the fix is a per-call counter in the
  name (`crates/nvs-cli/tests/openapi.rs`'s `document`). [until: gone crates/nvs-cli/tests/openapi.rs:CARGO_TARGET_TMPDIR]
