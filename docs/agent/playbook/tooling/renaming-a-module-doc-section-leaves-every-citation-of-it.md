- **Renaming a module-doc section leaves every citation of it pointing at a heading that no longer
  exists, and nothing `nv verify` runs reads a `§ *Title*` reference.** Splitting `route.rs`'s and
  `serve.rs`'s headings stranded `crates/nvs-server/src/lib.rs:88`, and a goal's prose quotes
  `file.rs:NN-NN` spans that move when the block they name moves. Grep the
  old title *and* the old line span across `crates/` and `docs/agent/goals/` in the same call that
  makes the edit.
  [until: gone crates/nvs-server/src/lib.rs:§]
