- **`nv splice` cannot tell two byte-identical blocks in one file apart.** Five identical
  `Inst { … }` literals in `crates/nvs-ir/src/lower/control.rs` are one anchor five times over, and
  a patch naming it edits the first and leaves four. Widen each anchor with the lines around it, or
  — when every occurrence wants the same edit — give that one block to the Edit tool's `replace_all`
  and keep the rest of the run in the patch. [until: exists tools/nv/cmd/splice.ts:--occurrence]
