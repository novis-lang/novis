- **A reserved word has no accessor for its own spelling.** `Keyword::from_lowercase` in
  `crates/nvs-syntax/src/token.rs` is the only table, and it maps the spelling *to* the variant rather
  than back. A list of spellings written anywhere else is therefore a
  copy — write it as `&[&str]` and guard it with a test that every entry round-trips through
  `from_lowercase`, which is what `nvs_lsp::completion`'s keyword lists do.
  [until: exists crates/nvs-syntax/src/token.rs:pub fn name]
