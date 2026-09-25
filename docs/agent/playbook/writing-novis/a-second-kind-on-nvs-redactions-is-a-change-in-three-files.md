- **A second `kind` on `nvs/redactions` is a change in three files, and the two that are not the
  server fail quietly.** The client conceals every kind it is handed, so a marker spelling added to
  `crates/nvs-lsp/src/redactions.rs` alone bars an identifier until
  `editors/vscode/src/concealment.ts`'s `MARKERS` has been taught it, and
  `docs/reference/tools/40-editor.md` names the kinds in prose, so `docs/novis.md` goes stale under
  a check nobody edited. Write all three, then `bun nv reference --no-examples`.
  [until: gone editors/vscode/src/concealment.ts:const MARKERS]
