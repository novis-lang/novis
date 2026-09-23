- **A `note:` on the configuration-resolution path prints on every `nvs` run in this checkout, and a
  landed test pins that stream empty.** `crates/nvs-cli/tests/bundle.rs:172` asserts `nvs run` writes
  nothing to stderr, and `cargo test` runs it in `crates/nvs-cli`, which fails
  `rule:config/ownership-is-the-trust-boundary` on any Windows drive whose root grants
  `Authenticated Users` write. Carry the reason as a value for the command an operator typed to
  render, rather than printing from the resolver.
  [until: gone crates/nvs-cli/tests/bundle.rs:nothing extra on standard error]
