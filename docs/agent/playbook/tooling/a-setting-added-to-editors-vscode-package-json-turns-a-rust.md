- **A setting added to `editors/vscode/package.json` turns a Rust test red, not a TypeScript one.**
  `crates/nvs-lsp/tests/extension_reference.rs` reads the manifest against the settings table in
  `docs/reference/tools/40-editor.md` and asserts the two name the same keys with the same defaults,
  so a contribution that lands in the manifest and the headless suite alone fails the whole cargo
  gate on a change that touched no Rust. Add the table row in the same slice and then run `bun nv
  reference`, because that chapter is one of `docs/novis.md`'s sources and `--check` over it
  is an acceptance check of its own. [until: gone crates/nvs-lsp/tests/extension_reference.rs]
