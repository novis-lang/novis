# Handoff

## State

**Goal 14, stage 9: every cursor row is full, and the document-wide half is short one construct.**
`nvs lsp-test tests/lsp/` is at `188 passed, 0 failed` against the goal's floor of 160, and
`--coverage` prints 26 constructs. `hover`, `definition`, `completion`, `selectionRange` and
`codeAction` each hold all 26 of their cells.

**`Echo` is closed and `ClassConstAccess` cannot be closed as the server stands.** A whole-document
answer now reaches the echo statement through `tests/lsp/diagnostics/`'s
`an-echo-with-no-argument-is-reported-inside-the-statement.lspt` — a parse error at the empty span
inside the statement, which is inside no child of it. No such position exists for a class constant:
every diagnostic about one is reported at the class expression
(`crates/nvs-hir/src/members.rs:1058`), which is its own `ConstFetch` node;
`crates/nvs-lsp/src/semantic.rs:852` emits a token for that receiver and none for the member;
a malformed `Config::` reports *past* the access's end and credits `Echo`; and `documentSymbol`,
`foldingRange`, `documentLink` and `redactions` name no expression's member half at all. Probed,
not assumed.

**The gate is unwritten on purpose.** `every_request_answers_every_construct` cannot assert
`crates/nvs-lsp/src/coverage.rs:65`'s document-wide claim until the item below is decided, and
writing it against a weaker claim would settle that decision by default.

## Next group

**Stage 9: the last construct, then the gate** — one file set: `crates/nvs-lsp/src/semantic.rs`,
`crates/nvs-lsp/src/coverage.rs`, `tests/lsp/semantic/` and a new `crates/nvs-lsp/tests/coverage.rs`.

- [ ] **A document-wide answer reaches `ClassConstAccess`** — `rule:ide/lspt-coverage-is-inferred`.
      Recommended: emit `enumMember` for the member half at `crates/nvs-lsp/src/semantic.rs:852`
      where the checker recorded `ExprInfo::EnumCase` — exact rather than a guess, and the same
      table read the `DefaultLibrary` modifier already makes. It rewrites that module's recorded
      silence for the enum half only; a plain class constant stays silent, LSP having no name for
      one. Re-freeze any `tests/lsp/semantic/` case holding an enum-case read, saying so in the
      commit message.
- [ ] **`every_request_answers_every_construct` reads the matrix and names each empty cell** — new
      `crates/nvs-lsp/tests/coverage.rs`, over `crates/nvs-lsp/src/coverage.rs:232`'s `constructs()`,
      `crates/nvs-lsp/src/coverage.rs:247`'s `count()` and
      `crates/nvs-lsp/src/coverage.rs:256`'s `reached_nothing()`. If the item above is refused, the
      other repair is to restate `crates/nvs-lsp/src/coverage.rs:65`'s document-wide half so it
      demands a non-empty row per request and *reports* a construct only cursors reach.

## Backlog

- Hover on a declaration's own name answers `none`; the `///` run above it is reachable only from a
  reference. Frozen as current behaviour; whether it should answer is `docs/plan/m4b.md`'s question.
- A cursor inside a string literal is offered the statement words beside the variables an
  interpolation slot wants — `crates/nvs-lsp/src/completion.rs`'s *Known gaps*, sixth entry.
- Eight cases fill no cell at all and are listed by name under the matrix; each is a deliberate
  claim about a position inside no node, `crates/nvs-lsp/src/coverage.rs:37`.
