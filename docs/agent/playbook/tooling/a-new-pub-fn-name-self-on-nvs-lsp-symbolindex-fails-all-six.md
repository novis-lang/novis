- **A new `pub fn name(&self …)` on `nvs_lsp::SymbolIndex` fails `every_reader_queries_the_one_index`,
  even when no editor feature calls it.** The test reads every read-only method in `index.rs` and asks
  that the set equal the readers' queries, so a count for a CLI caller reads as one more reader.
  A method that looks no name up goes into `BOOKKEEPING` in `crates/nvs-lsp/tests/index.rs`, with its
  caller named in the comment above it; a method that does look a name up is a reader, and it is a
  decision for `rule:ide/five-features-are-one-reference-index`. `until: gone crates/nvs-lsp/tests/index.rs:const BOOKKEEPING`
