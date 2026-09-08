# Handoff

## State

**Goal 14, stage 9: three of the five cursor rows have no empty cell; the gate over the matrix is
still unwritten.** `nvs lsp-test tests/lsp/` is at `145 passed, 0 failed` against the goal's floor
of 160, and `--coverage` prints 26 constructs. `hover`, `definition` and `codeAction` now hold all
26 of their cells; `completion` holds 7 and `selectionRange` holds 3.

**The gate's definition is decided and recorded**, in `crates/nvs-lsp/src/coverage.rs:46`
§ *Decision: what "no empty cell" can mean*: a request asked at a cursor owes the whole vocabulary,
the six document-wide requests owe it between them, and the vocabulary is whatever the corpus
reached. The document-wide half is short exactly two constructs — no whole-document answer anywhere
reaches `ClassConstAccess` or `Echo`, which are in the vocabulary through cursor rows alone.

**Most of the new cases freeze `none`, deliberately.** Hover and definition both read the checker's
expression table, so a declaration's own name reaches nothing and answers nothing; `codeAction`
answers nothing wherever no diagnostic carries a suggestion, which is
`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows` pinned everywhere the two
fixes are not. Freezing that is what makes a row honest rather than sparse.

## Next group

**Stage 9: the two cursor rows still short, then the gate** — one file set: new `.lspt` cases under
`tests/lsp/completion/` and `tests/lsp/selection/`, and a new `crates/nvs-lsp/tests/coverage.rs`.
Run `./target/debug/nvs.exe lsp-test tests/lsp/ --coverage` first: it prints exactly which cells are
empty, so nothing has to be derived.

- [ ] **The `completion` row reaches every construct in the vocabulary** —
      `rule:ide/lspt-coverage-is-inferred`, `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
      Nineteen cases under `tests/lsp/completion/`; what a cursor is offered is
      `crates/nvs-lsp/src/completion.rs:183`, and a construct that offers nothing freezes `none`.
      The documents to copy are the hover row's, one construct each.
- [ ] **The `selectionRange` row does the same** — same two rules. Twenty-three cases under
      `tests/lsp/selection/`; the answer is the index's ancestor chain,
      `crates/nvs-lsp/src/selection.rs:44`, rendered a line per ancestor by
      `crates/nvs-lsp/src/render.rs:433`.
- [ ] **A document-wide answer reaches `ClassConstAccess` and `Echo`** —
      `rule:ide/lspt-coverage-is-inferred`. One `semanticTokens` or `documentSymbol` case whose
      answer names them closes the second half of the gate's claim,
      `crates/nvs-lsp/src/coverage.rs:65`.
- [ ] **`every_request_answers_every_construct` reads the matrix and names each empty cell** — new
      `crates/nvs-lsp/tests/coverage.rs`, over `crates/nvs-lsp/src/coverage.rs:232`'s `constructs()`,
      `crates/nvs-lsp/src/coverage.rs:247`'s `count()` and
      `crates/nvs-lsp/src/coverage.rs:256`'s `reached_nothing()`.

## Backlog

- Hover on a declaration's own name answers `none`; the `///` run above it is reachable only from a
  reference. Frozen as current behaviour; whether it should answer is `docs/plan/m4b.md`'s question.
- The corpus is 145 cases against the goal's floor of 160 — the `nvs-suite` check in
  `docs/agent/loop-goal.toml`.
- Eight cases fill no cell at all and are listed by name under the matrix; each is a deliberate
  claim about a position inside no node, `crates/nvs-lsp/src/coverage.rs:37`.
