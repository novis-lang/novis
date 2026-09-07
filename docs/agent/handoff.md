# Handoff

## State

**Goal 14 stage 4 has opened: `documentSymbol` is answered end to end.**
`crates/nvs-lsp/src/symbols.rs`'s `for_document` walks the entry file's `Analysed::loaded` statements into
the `lsp_types::DocumentSymbol` tree `crates/nvs-lsp/src/render.rs:370`'s `outline` renders — classes,
interfaces, enums and their members, namespaces, type aliases, and the top-level `function` and `const`
`rule:classes/no-free-functions-or-constants` refuses, because an outline says what the file holds rather
than what the checker allows. A declaration whose name covers no bytes contributes nothing; that module's
doc owns the reasoning for both, and for the `TypeParameter` kind a type alias lands under.

**`crates/nvs-lsp/src/server.rs:148`'s `answer` is the request seam.** One place matches a method name,
with three outcomes and no fourth: an arm's answer, `MethodNotFound` outside
`rule:ide/the-request-set-is-closed`'s list, and `InvalidParams` for a payload that will not deserialize.
`crates/nvs-lsp/src/suite.rs:179`'s `answer` is the same seam for a `.lspt` case and now has two arms.

**`nvs lsp-test tests/lsp/` reports `9 passed, 0 failed`**, four of them under `tests/lsp/symbols/`.

**No cursor request can land yet.** `crates/nvs-lsp/src/document.rs:315` calls `parse_file`, so an
`Analysed` carries statements and no `Parsed` — neither the trivia nor `crates/nvs-syntax/src/index.rs:103`'s
`SyntaxIndex` reaches this crate, and `rule:ide/the-index-answers-the-cursor` is what hover, definition,
completion and `selectionRange` are all waiting on.

## Next group

**Stage 4: the two remaining requests that need no cursor and no types** — one file set:
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/suite.rs`, a new module beside
`crates/nvs-lsp/src/symbols.rs`, with `crates/nvs-lsp/src/render.rs` read only. Both are projections of
what one analysis already holds, which is why they come before the index work above.

- [ ] **`foldingRange`, from the spans the statements already carry.** A walk of
      `crates/nvs-lsp/src/document.rs:266`'s `Analysed::loaded` entry file for every construct with a
      body — a class or interface body, a method, a block statement — rendered by
      `crates/nvs-lsp/src/render.rs:398`'s `folding_range`, with the arm going into
      `crates/nvs-lsp/src/server.rs:148` and `crates/nvs-lsp/src/suite.rs:179`.
      `rule:ide/the-request-set-is-closed`.
- [ ] **`documentLink`, out of the require edges the graph walk already resolved.**
      `crates/nvs-hir/src/requires.rs:154`'s `Loaded::requires` is the `(Span, SourceId)` pair per
      `require` literal; the range is the span and the target is the path, which the runner resolves back
      to the case's own spelling as `crates/nvs-lsp/src/render.rs:404`'s `Link` wants
      (`crates/nvs-lsp/src/suite.rs:247`'s `store` is where the materialised directory is known).
      `rule:ide/the-rendering-has-one-home`.
- [ ] **Their `.lspt` cases, under `tests/lsp/folding/` and `tests/lsp/links/`.** Each request ships its
      own, and at least one document per request that does not parse —
      `crates/nvs-lsp/src/suite.rs:179` is where an unanswered request still fails a case by name.

## Backlog

- The cursor requests need `Analysed` to carry `Parsed` rather than `Vec<Stmt>` — `crates/nvs-lsp/src/document.rs:315`.
- `nvs lsp-test --coverage` and `every_request_answers_every_construct` are unwritten; `rule:ide/lspt-coverage-is-inferred` owns them.
- `render.rs`'s outline has no column for `DocumentSymbol::detail`, so the server sends none — `crates/nvs-lsp/src/symbols.rs`'s module doc.
- Seven `Response` variants still have no producer; `docs/agent/carried-gaps.md` is where this goes if the goal switches.
