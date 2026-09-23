- **A structural "exactly one construction site" test cannot count the type's own name.** `nvs-lsp`'s
  `SymbolIndex` is built through `Self::default()` rather than a struct literal and is re-exported by
  name from `lib.rs`, so counting `SymbolIndex {` finds only the declaration and counting `SymbolIndex`
  finds the export list — neither is a site where anything is built. Count the **private** entry type
  the site fills in (`Indexed {`), which no other module can name, and exempt `lib.rs` explicitly, which
  is what `crates/nvs-lsp/tests/index.rs` does. [until: gone crates/nvs-lsp/src/index.rs:struct Indexed]
