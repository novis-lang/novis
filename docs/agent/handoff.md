# Handoff

## State

**Goal 14, stage 9's two code actions are closed.** `textDocument/codeAction` answers:
`crates/nvs-lsp/src/actions.rs` is the translation, `server.rs` dispatches the method and builds the
`WorkspaceEdit`, `suite.rs` runs a `codeAction` case at the empty range under its `<|>`, and
`render.rs` freezes one as `L:C-L:C kind title -> "replacement"`.

**Nothing here computes a fix.** An action is one `nvs_diagnostics::Suggestion` a diagnostic was
already carrying, so the boundary of
`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows` is structural rather than a
list: a fix the checker would have to compute has nothing to be translated from. The two that ship
are the casing rename, which `nvs_syntax::casing` already carried, and the legacy cast, which now
carries `$n as int` as an **unsafe** fix — `(int)$n` truncates where `as` throws, so `nvs convert`
must not apply it silently while an editor may offer it.

The gate an editor sees is the gate an action sits behind (`diagnostics::phase_gated` is applied
here too), and the kind is decided by the client: `editor.codeActionsOnSave` asks for
`source.fixAll.nvs` and a light bulb for `quickfix`, and one translation answers both
(`actions::Kind`).

`nvs lsp-test tests/lsp/` reports `83 passed, 0 failed` against the goal's floor of 160. Stage 9's
remaining check is the coverage matrix, which is the group below; stage 4's case floor is fed by
every slice after it.

## Next group

**Stage 9: the coverage matrix has no empty cell** — one file set: a new
`crates/nvs-lsp/src/coverage.rs`, with `crates/nvs-lsp/src/suite.rs` and
`crates/nvs-cli/src/main.rs` edited, and `crates/nvs-lsp/src/case.rs`'s `Request::ALL` read as the
matrix's one axis.

- [ ] **A case's coverage is inferred, not registered** — `rule:ide/lspt-coverage-is-inferred`,
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`. The other axis is the construct the
      cursor resolved to, off `Analysed::index`, and a document-wide request covers every construct
      its answer named. The seam is the runner's own answer function,
      `crates/nvs-lsp/src/suite.rs:187`, which already holds the case and the analysis together.
- [ ] **`nvs lsp-test --coverage` prints the matrix** — `rule:ide/lspt-coverage-is-inferred`. The
      flag goes on the subcommand at `crates/nvs-cli/src/main.rs:449` and its dispatch at
      `crates/nvs-cli/src/main.rs:903`; printing the matrix replaces the summary line rather than
      adding to it, since the loop parses `N passed, M failed`.
- [ ] **`every_request_answers_every_construct` fails naming each empty cell** — the acceptance
      check at `docs/agent/loop-goal.toml:5303`. A `-p nvs-lsp` test over `tests/lsp/`, and the
      cells it names are the cases the rest of the goal still owes. Landing it red is not an option
      the loop can carry, so the cases that fill it land with it.

## Backlog

- `nvs/redactions` does not reach a parameter's default or a property declaration's default —
  `crates/nvs-lsp/src/redactions.rs`'s module doc names both; closing it is a `nvs-types` change.
- The `.lspt` corpus is at 83 of the goal's floor of 160 — every request slice ships its own.
- `documentHighlight` and the four write actions stay at M10 — `rule:ide/five-features-are-one-reference-index`.
- The VS Code client, the grammar and the legend's client half are goal 15's, not this goal's.
