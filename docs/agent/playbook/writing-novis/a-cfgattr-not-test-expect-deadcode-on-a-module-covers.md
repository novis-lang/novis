- **A `#[cfg_attr(not(test), expect(dead_code))]` on a module covers nothing the `cfg(test)` build
  adds to it.** `crates/nvs-cli/src/service.rs`'s `registration` carries one, so an applier landed
  ahead of its caller compiles silently under `cargo build` and then warns under `cargo clippy
  --all-targets`, which is the only leg that builds the test copy at all. Give each such item its
  own `#[cfg_attr(test, expect(dead_code, reason = …))]`, and reach for `cargo clippy --all-targets`
  rather than `cargo build` while a seam still has no caller.
  [until: gone crates/nvs-cli/src/service.rs:the subcommands that reach this seam]
