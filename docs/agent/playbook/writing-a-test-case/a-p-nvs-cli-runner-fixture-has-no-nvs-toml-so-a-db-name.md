- **A `-p nvs-cli` runner fixture has no `nvs.toml`, so a `[db.<name>]` block built in Rust skips
  the boot-time path resolution.** `nvs_config::db` resolves `tls_ca_file` against its file's
  directory, and a tree assembled in Rust has no file, so a relative `"tests/db/ca.crt"` resolves
  against whatever directory `cargo test` chose. Build such paths from `CARGO_MANIFEST_DIR`;
  `runner.rs`'s `compose_postgres` is the shape. [until: gone crates/nvs-cli/src/runner.rs:CARGO_MANIFEST_DIR]
