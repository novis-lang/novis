- **`cargo test -p nvs-cli --lib` is `error: no library targets found in package`, and the unit
  tests it was meant to run are in the bin target.** `nvs-cli` has no `lib.rs`, so a module's
  `#[cfg(test)] mod tests` runs under `cargo test --bin nvs <filter>`, and
  `crates/nvs-cli/tests/*.rs` drive the built binary instead of linking to anything. The same is
  true of every binary-only crate here. [until: exists crates/nvs-cli/src/lib.rs]
