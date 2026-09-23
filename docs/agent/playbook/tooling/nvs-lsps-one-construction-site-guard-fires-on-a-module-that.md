- **`nvs-lsp`'s one-construction-site guard fires on a module that merely *names* `SymbolIndex`, the
  `use` line included.** `crates/nvs-lsp/tests/index.rs`'s
  `the_crate_has_exactly_one_symbol_index_construction_site` closed that name to `index.rs`,
  `lib.rs` and `server.rs`, so the first feature module to read the index — completion — failed a
  test whose message is about *building* one. Widen it by asserting that a module outside those
  three names the type only behind a `&` and in an import, which is what the test's own comment
  already claims, rather than by adding the module to the list.
  [until: gone crates/nvs-lsp/tests/index.rs:names SymbolIndex other than behind]
