# Handoff

## State

**Goal 14 answers four of the ten requests, and the resilient parse is what the fourth rests on.**
`crates/nvs-lsp/src/document.rs:338` calls `nvs_syntax::parse` rather than `parse_file`, so an
`Analysed` carries the entry document's whole `Parsed` — the same statements plus the trivia
(`crates/nvs-lsp/src/document.rs:274`) and the `SyntaxIndex`
(`crates/nvs-lsp/src/document.rs:279`). A required file keeps the strict parse, because a cursor is
only ever in the open one. `rule:ide/one-grammar-one-tree`.

**`selectionRange` is `crates/nvs-lsp/src/selection.rs` and nothing else**: the index's ancestor
list turned into the wire's parent-linked chain, unedited. An offset inside no node answers nothing;
the server sends the empty range at that position so its array stays paired with the client's, and a
`.lspt` case renders it `none`. `foldingRange` gained the one kind its walk could not reach — a run
of comment lines, out of that trivia, with the two rules for what joins a run in
`crates/nvs-lsp/src/folding.rs`'s module doc.

**`nvs lsp-test tests/lsp/` reports `27 passed, 0 failed`**, against the goal's floor of 160.

**Stage 7's four projections are done; `semanticTokens/full` is the fifth and waits on the same
thing stage 6 does.** `crates/nvs-lsp/src/document.rs:368` still drops `nvs_types::ExprTypeTable` on
the floor, and every remaining request wants it.

## Next group

**Stage 6: the cursor requests that need a name and a type** — one file set:
`crates/nvs-lsp/src/document.rs`, `crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/suite.rs`,
`crates/nvs-lsp/src/render.rs`, and two new siblings of `crates/nvs-lsp/src/selection.rs`, with
`crates/nvs-hir/src/symbol.rs` read only. Each is the index plus one table that already exists.

- [ ] **`Analysed` keeps the expression types it computes and throws away.** The type phase at
      `crates/nvs-lsp/src/document.rs:368` fills an `ExprTypeTable` and drops it with the comment
      saying which request would keep it; that request is here. Keep it and the `TypeInterner` it is
      read against beside the index on `Analysed` (`crates/nvs-lsp/src/document.rs:279`) — a type
      without its interner is an id nothing can spell. `rule:ide/an-open-document-is-its-own-entry-point`.
- [ ] **`definition`, for a declared type name.** `nvs_hir::Symbol::decl_span`
      (`crates/nvs-hir/src/symbol.rs:47`) is the answer — the declared name's own span, anywhere in
      the resolved graph. The open question to settle first: the index answers a kind and a span,
      not a name (`crates/nvs-lsp/src/selection.rs:44` is the whole of how a request reads it), so
      the identifier has to come out of the entry's text at that span. New
      `crates/nvs-lsp/src/definition.rs`, arms at `crates/nvs-lsp/src/server.rs:156` and
      `crates/nvs-lsp/src/suite.rs:182`, rendered as `crates/nvs-lsp/src/render.rs:68`'s `Place`,
      cases under `tests/lsp/definition/`. `rule:ide/the-index-answers-the-cursor`.
- [ ] **`hover`, starting with a declaration's doc comment.** The `DocComment` trivia
      `Analysed` now carries is the run directly above a declaration, and
      `crates/nvs-lsp/src/render.rs:270` already freezes a `Hover` as its markdown verbatim. The
      `Core`-member signature row is the same handler's second source and can follow in its own
      slice. `rule:tooling/doc-comment-is-three-slashes`.

## Backlog

- `semanticTokens/full` with ADR 0099 § 4's legend, including `tainted`/`secret` — stage 7's fifth,
  and the second reader of the kept `ExprTypeTable`. `docs/agent/loop-goal.md` § *Stage 7*.
- `completion` — the third of stage 6's group, and the largest. `docs/agent/loop-goal.md` § *Stage 6*.
- `nvs/redactions`, which is the driver's currently-failing acceptance check.
  `docs/agent/loop-goal.md` § *Stage 8*.
- The two code actions and the boundary test. `docs/agent/loop-goal.md` § *Stage 9*.
- The latency guard on a 1,000-line document. `docs/agent/loop-goal.md` § *Stage 10*.
- `#region` folds: a marker convention nothing in this language has decided.
  `crates/nvs-lsp/src/folding.rs`'s module doc says why it is not built.
