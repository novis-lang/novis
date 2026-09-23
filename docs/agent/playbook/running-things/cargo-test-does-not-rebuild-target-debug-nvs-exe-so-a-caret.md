- **`cargo test` does not rebuild `target/debug/nvs.exe`, so a caret you read out of it after a
  test-only build is the previous commit's.** The driver builds the binary at the commit a session
  *starts* from and `cargo test` builds only test targets, so `nvs check` keeps rendering the old
  diagnostic while the suite already sees the new one — which reads as a bug in the change you just
  made. Run `cargo build` before reading a diagnostic out of `nvs.exe` whenever you have touched Rust
  this session. [until: reviewed 2026-10-16]
