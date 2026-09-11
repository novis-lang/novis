# Handoff

## State

**Goal `workspace-index`, stage 3 is open and four of its five readers have landed.** The index has a
home in `serve` at `crates/nvs-lsp/src/server.rs:161`, and what it is built over is now the client's to
choose: `crates/nvs-lsp/src/settings.rs` reads `nvs.check.scope` and `nvs.codeLens.enable` off
`initializationOptions`, plus the workspace root off `workspaceFolders`, and an unreadable value takes
its default rather than refusing `initialize`.

`textDocument/references` and `textDocument/documentHighlight` are answered from the index, and
**unused-member dimming** is the third reader: `SymbolIndex::unused_private` at
`crates/nvs-lsp/src/index.rs:311` is the query, `diagnostics::dimming` renders the `Unnecessary` tag, and
`publish` appends it. It is empty under the default scope, which is `rule:ide/check-scope-defaults-to-open-documents`
rather than a shortcut. A `Declaration` now records the visibility it was written with, which closed
`index.rs`'s second known gap.

Stage 3's gate still owes `all_five_readers_query_the_one_index`, and it cannot pass until CodeLens and
type hierarchy exist — they are readers four and five, and the test counts five. Nothing is blocked. The
goal's one ADR is still unopened.

**Nothing in the extension declares or sends either setting**, so no developer can reach them yet:
`editors/vscode/package.json` contributes neither, and `extension.ts` passes no `initializationOptions`.
That is the extension slice in the backlog, and `crates/nvs-lsp/tests/extension_reference.rs` will want
the chapter updated in the same commit.

## Next group

**Stage 3: the last two readers, then the test that there are five** — one file set:
`crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/index.rs`,
`crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/tests/index.rs`.

- [ ] **CodeLens, behind `nvs.codeLens.enable`.** Declare `codeLensProvider` at
      `crates/nvs-lsp/src/capabilities.rs:152` and dispatch from `answer` at
      `crates/nvs-lsp/src/server.rs:213`, which today takes `&Documents` and `&SymbolIndex` and will need
      the setting too — `serve` already holds it at `crates/nvs-lsp/src/server.rs:148`. The lens counts
      `SymbolIndex::occurrences` and reads `declarations_in`, so it walks no front end.
      `rule:ide/five-features-are-one-reference-index` is what a lens shows.
- [ ] **Type hierarchy.** `textDocument/typeHierarchy`'s three requests (`prepare`, `supertypes`,
      `subtypes`) off the same index, at `crates/nvs-lsp/src/server.rs:213`. Supertypes and subtypes are
      an `extends`/`implements` edge the declaration walk at `crates/nvs-lsp/src/index.rs:512` does not
      record yet, so this slice adds it there beside the visibility field.
      `rule:classes/interface-default-methods` and `rule:classes/delegation-by-field` are the two shapes
      a reader most needs shown rather than reconstructed.
- [ ] **`all_five_readers_query_the_one_index`.** The stage's gate, beside the construction-site test at
      `crates/nvs-lsp/tests/index.rs:137`: five features, one index, and the count is the assertion.
      Note that `the_crate_has_exactly_one_symbol_index_construction_site` restricts which *files* may
      name `SymbolIndex` to `index.rs`/`lib.rs`/`server.rs`, so a lens module of its own either takes
      `&SymbolIndex` under a widened check or the arms stay in `server.rs`.
      `rule:ide/five-features-are-one-reference-index` names all five.
- [ ] **An `.lspt` case per added request.** A directory under `tests/lsp/` each, gated by
      `nvs lsp-test tests/lsp/ --coverage`, which is stage 3's second check. `references`,
      `documentHighlight`, `codeLens` and `typeHierarchy` each need one, and every case's
      `--EXPECT--` compares against `crates/nvs-lsp/src/render.rs:135`, which is the one home for the
      spelling — a case that seems to need its own has found a gap there.
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case` is the format.

## Backlog

- The extension contributes neither new setting and sends no `initializationOptions` —
  `editors/vscode/package.json`, `editors/vscode/src/extension.ts`, plus the chapter
  `crates/nvs-lsp/tests/extension_reference.rs` reads.
- `nvs.checkWorkspace`, the on-demand workspace pass — `rule:ide/check-scope-defaults-to-open-documents`.
- A multi-root workspace takes its first folder only — `crates/nvs-lsp/src/settings.rs:@root_of`.
- The enum-case occurrence asymmetry, `index.rs`'s remaining known gap — owner: unowned.
- This goal's one ADR is unopened; `docs/agent/loop-goal.md` § *Standing decisions* says what it covers.
