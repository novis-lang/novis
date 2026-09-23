- **The `unreachable from source` phrase is counted from the line the `Fault::` sits on, not from
  its statement.** The gate wants it within 8 lines, so a builder chain between the comment and the
  `Fault::fatal` pushes it out of `DECLARATION_WINDOW`. Put the phrase on the comment's last line
  and run `cargo test --test conformance_coverage every_error_path` after a batch.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION_WINDOW]
