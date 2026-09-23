- **Widening a `nvs_types` enum that crosses into `nvs_runtime` breaks a third crate no file set
  named.** `nvs-cli/src/main.rs` maps `nvs_types::commands::ArgConv` onto the runtime's by an
  exhaustive hand-written `match`, and `capture_conv` does the same for `CaptureConv`, so one variant
  replaced in the checker is a compile error two crates away. Grep `nvs_types::` across
  `crates/nvs-cli/src/main.rs` before pricing such a slice, and decide the bridge's answer for the new
  arm in that slice rather than from the build.
  [until: gone crates/nvs-cli/src/main.rs:nvs_types::commands::ArgConv::]
