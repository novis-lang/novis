- **Moving a `<|>` cursor inside an existing `.lspt` case empties a cell of
  `crates/nvs-lsp/tests/coverage.rs`'s matrix, naming a construct the case still contains.** The
  matrix is keyed on the node the *cursor* sits in, and a qualifier is a node of its own:
  `Sta<|>tus::Draft` covers `ConstFetch`, `Status::Dr<|>aft` covers `ClassConstAccess`. Where a case
  is the only one covering its cell, change what it expects rather than where its cursor sits.
  [until: gone crates/nvs-lsp/tests/coverage.rs:empty cell]
