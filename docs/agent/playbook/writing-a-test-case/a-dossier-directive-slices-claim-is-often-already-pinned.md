- **A dossier `directive:` slice's claim is often already pinned elsewhere in the same crate, and the
  pack points only at the registry row.** `[trace] sample`'s bounds are an `export.rs` unit test and
  `[opcache]`'s revalidation semantics are two cases in `crates/nvs-config/tests/snapshot.rs`, so a
  registry case written from the rule alone re-asserts one of those under a new name. Grep the
  crate for the key before writing, and assert the field those cases do not read — the blanket over
  the keys a block *accepts* for one whose semantics are pinned, `Apply` for one whose class is.
  [until: reviewed 2026-09-18]
