- **A `macro_rules!` that lands with no call site turns `nv verify` red at clippy, not at the
  build.** `cargo build` reports `unused macro definition` as a warning and exits green, while
  `nv verify` runs clippy with `-D warnings` and fails on that same line, so a keystone macro
  written in one slice and first used in the next never gets through. Land the macro with its first
  real call site in the same slice — for `guarded_by!` that meant closing one refusal site and
  lowering `crates/nvs-ir/tests/refusals.rs`'s `CEILING` alongside it.
  [until: gone crates/nvs-ir/tests/refusals.rs:CEILING]
