- **A field added to `nvs_types::Env` builds clean and fails `--all-targets`.** There are three
  construction sites and one (`crates/nvs-types/src/lower.rs`) is inside a `#[cfg(test)]` module, so
  `cargo build -p nvs-types` is green while `cargo check -p nvs-types --all-targets` is the first
  thing that reports `E0063: missing field`. The two real sites are `check.rs`'s per-file loop and
  `signatures.rs`, which wants a scratch value because its pass runs before anything fills the new
  table. [until: gone crates/nvs-types/src/signatures.rs:Env {]
