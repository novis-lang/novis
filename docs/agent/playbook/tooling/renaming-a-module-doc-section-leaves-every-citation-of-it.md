- **Renaming a module-doc section leaves every citation of it pointing at a heading that no longer
  exists, and nothing `verify.py` runs reads a `§ *Title*` reference.** Splitting `route.rs`'s and
  `serve.rs`'s headings stranded `crates/nvs-server/src/lib.rs:88`, and moving a gap block moved the
  `file.rs:NN-NN` spans that `docs/agent/goals/57-m7-server-surface.md:67` quotes for it. Grep the
  old title *and* the old line span across `crates/` and `docs/agent/goals/` in the same call that
  makes the edit.
  [until: reviewed 2026-09-14]
