# Handoff

## State

**Goal 14, stage 9: coverage is inferred and printable; the gate over it is not written.**
`crates/nvs-lsp/src/coverage.rs` is the inference, `suite::answer` now makes the analysis once and
hands it to every arm, `suite::Outcome` carries the matrix beside the count, and
`nvs lsp-test tests/lsp/ --coverage` prints it in place of the summary line.

**The gate's definition is decided and recorded**, in `crates/nvs-lsp/src/coverage.rs:46`
§ *Decision: what "no empty cell" can mean*. The literal cross product is impossible — a
`documentLink` answer names a `require`'s path literal and nothing else, so its cell at `ClassDecl`
is empty in every corpus that could exist. So: a request asked at a cursor owes the whole
vocabulary, the six document-wide requests owe it between them, and the vocabulary is whatever the
corpus reached. Nothing is hand-listed, which is
`rule:ide/lspt-coverage-is-inferred`'s actual requirement.

**What the matrix says now**: 26 constructs, `nvs lsp-test tests/lsp/` at `83 passed, 0 failed`
against the goal's floor of 160. The five cursor rows hold 26 of their 130 cells; the document-wide
rows cover every construct between them except `ClassConstAccess` and `Echo`. Those 106 cells are
the cases the goal still owes, and they are the group below.

## Next group

**Stage 9: the cells the matrix names** — one file set: new `.lspt` cases under `tests/lsp/`, a new
`crates/nvs-lsp/tests/coverage.rs`, and `crates/nvs-lsp/src/coverage.rs:196`'s `Matrix` read for
`constructs()`, `count()` and `reached_nothing()`. Run
`./target/debug/nvs.exe lsp-test tests/lsp/ --coverage` first: it prints exactly which cells are
empty, so nothing has to be derived.

- [ ] **The `hover` and `definition` rows reach every construct in the vocabulary** —
      `rule:ide/lspt-coverage-is-inferred`, `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
      About twenty cases each under `tests/lsp/hover/` and `tests/lsp/definition/`; most freeze
      `none`, which is the answer at a construct that declares nothing, and freezing it is what
      makes the row honest. The construct a case lands on is the innermost node at its `<|>` —
      `crates/nvs-lsp/src/coverage.rs:88`.
- [ ] **The `completion`, `selectionRange` and `codeAction` rows do the same** — same rule, same
      shape, under `tests/lsp/completion/`, `tests/lsp/selection/` and `tests/lsp/actions/`, with
      the cursor arms at `crates/nvs-lsp/src/suite.rs:266` deciding what each is asked. Every
      `codeAction` case but the two fixes freezes `none`, which is
      `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows` pinned everywhere it does
      *not* fire.
- [ ] **`every_request_answers_every_construct` reads the matrix and names each empty cell** — the
      acceptance check at `docs/agent/loop-goal.toml:5308`. A `-p nvs-lsp` test in a new
      `crates/nvs-lsp/tests/coverage.rs` running `suite::run(&["tests/lsp"], .., Report::Coverage)`
      and asserting the two claims at `crates/nvs-lsp/src/coverage.rs:46`. It lands green, so the
      last two document-wide cells (`ClassConstAccess`, `Echo`) land with it.

## Backlog

- Stage 10's latency bound on a 1,000-line document — `docs/plan/m4b.md`, `M4B:verify`.
- The VS Code extension and the grammar are goal 15's, not this one's.
- `tests/lsp/README.md` does not mention `--coverage`; the rule fragment already does.
