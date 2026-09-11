# Handoff

## State

**Goal `workspace-index`, stage 3: `references` is `.lspt` vocabulary.** Twenty-eight cases under
`tests/lsp/references/` — one per construct in the matrix, plus a cross-file pair and an enum read —
and the corpus is 223 passing with every cell of the `references` row filled.

A case is answered by `crate::server`'s `references_of_case`, which builds an index over the
directory the runner materialised the case into exactly as `lenses_of_case` does, then asks
`uses_of`. That query is what `textDocument/references` was already answering, split out of the wire
handler so that reading and converting a position is the only thing the wire half adds.
`Response::References(Vec<Place>)` renders one `file:L:C` per line, sorted by path and then position,
because the answer is the index's declaration side and its occurrence side put end to end across
every file that holds a use.

**Two gaps the cases found, now `crates/nvs-lsp/src/index.rs`'s `# Known gaps` 4 and 5.** A cursor on
a declaration's own name resolves to no symbol, so `references` answers `none` at the end a reader is
most likely to ask from; and an occurrence's span is the expression holding the name rather than the
name. The second is the one that has to close before `documentHighlight` freezes anything:
find-references survives a wide span because a reader reads the line, and a highlight box does not.

Nothing is blocked. The goal's one ADR is still unopened, and it still owes
`rule:ide/the-request-set-is-closed` the amendment naming M10's additions beside M4B's nine.

## Next group

**Stage 3: the occurrence span is narrowed, then `documentHighlight` becomes `.lspt` vocabulary** —
one file set: `crates/nvs-lsp/src/index.rs`, `crates/nvs-lsp/src/case.rs`,
`crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/src/suite.rs`, `crates/nvs-lsp/src/server.rs`,
`tests/lsp/references/`, `tests/lsp/highlight/`.

- [ ] **An occurrence is recorded at the name, not at the expression around it.** `site(path,
      node.span)` at `crates/nvs-lsp/src/index.rs:759` takes the whole node's span, and
      `crates/nvs-lsp/src/definition.rs`'s `named_at` is what already finds the name inside one.
      Re-freeze the six `tests/lsp/references/` expectations carrying a column, drop gap 5 from that
      module doc, and check `crates/nvs-lsp/tests/references.rs` still passes — it pins lines only.
      `rule:ide/five-features-are-one-reference-index`.
- [ ] **The variant and its rendering.** `Request::DocumentHighlight` in the enum at
      `crates/nvs-lsp/src/case.rs:57`, named in `ALL`, in `name()` and in `takes_cursor()`; a
      `Response::DocumentHighlight(Vec<Highlight>)` at `crates/nvs-lsp/src/render.rs:135` rendering
      one `L:C-L:C kind` per line, sorted, beside `fn reference`.
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case` is the format.
- [ ] **The arm.** A `pub(crate) fn highlights_of_case` beside `references_of_case` at
      `crates/nvs-lsp/src/server.rs:788`, and its caller in the match at
      `crates/nvs-lsp/src/suite.rs:264`. Every hit is `Text` and that is deliberate —
      `crates/nvs-lsp/src/server.rs:500`'s `document_highlight` says why.
- [ ] **Twenty-six cases under `tests/lsp/highlight/`**, one per construct in the matrix, the same
      sweep `tests/lsp/references/` and `tests/lsp/actions/` are. The row is a cursor row, so it owes
      the whole vocabulary — `crates/nvs-lsp/tests/coverage.rs:57` is the gate.

## Backlog

- Gap 4, a declaration's own name resolving to no symbol, is `definition::target_of`'s seam rather
  than the index's — `crates/nvs-lsp/src/index.rs` `# Known gaps` 4.
- `typeHierarchy` is the fourth added request still without `.lspt` vocabulary.
- The goal's one ADR is unopened; it amends `rule:ide/the-request-set-is-closed`.
- `nvs.codeLens.enable` off answers `None` rather than an empty list, and no case can write a
  setting — that branch is a `-p nvs-lsp` test's.
- Nothing in `editors/vscode` declares `nvs.check.scope` or `nvs.codeLens.enable` yet —
  `rule:ide/contributions-are-frozen-and-only-ever-added`.
