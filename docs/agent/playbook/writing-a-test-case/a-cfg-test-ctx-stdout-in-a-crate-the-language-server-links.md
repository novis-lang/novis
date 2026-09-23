- **A `#[cfg(test)]` `Ctx::stdout()` in a crate the language server links fails `nvs-lsp`, not the
  crate you wrote it in.** `crates/nvs-lsp/tests/stdout_policy.rs` greps the *source text* of every
  linked crate, so a unit test that builds a context the obvious way leaves `cargo test -p
  nvs-stdlib` green and turns two `-p nvs-lsp` tests red, naming
  `rule:ide/stdout-belongs-to-the-protocol` rather than the member you were writing. Build a test
  context as `Ctx::new(OutputSink::Sink)`, which is what every other `nvs-stdlib` unit test already
  does. [until: gone crates/nvs-lsp/tests/stdout_policy.rs]
