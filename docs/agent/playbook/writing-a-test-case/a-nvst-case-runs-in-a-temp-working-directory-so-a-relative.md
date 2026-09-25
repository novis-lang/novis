- **A `.nvst` case runs in a temp working directory, so a relative path resolves against *that*, and
  a case asserting a path *fails* passes either way.** A spawn case naming
  `"examples/isolate/capture.nvs"` was green because the file was missing there too. A case that
  needs a second file writes it as a `--FILE <name>--` section beside `--FILE--`: forward-slash
  relative paths, not `crates/nvs-test`'s `RESERVED_NAMES`. [until: gone crates/nvs-test/src/case.rs:--FILE]
