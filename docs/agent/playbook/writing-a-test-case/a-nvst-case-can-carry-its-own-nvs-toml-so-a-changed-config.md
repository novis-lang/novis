- **A `.nvst` case can carry its own `nvs.toml`, so a changed config spelling breaks tests a `grep`
  over `crates/` misses.** `--FILE nvs.toml--` writes one into the case's working directory, and the
  `router-url-absolute-*.nvst` cases do exactly that, one pinning the diagnostic that names the key
  in `--EXPECT--`. When you move a spelling that appears in a config file or a diagnostic, `grep
  -rl` over `tests/` and `examples/` in the same call as `crates/`; the message text is pinned both
  in the crate's `Fault` and in the case that catches it. [until: gone crates/nvs-test/src/case.rs:--FILE]
