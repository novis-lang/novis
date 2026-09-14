# Handoff

## State

**Goal `m4b-editor`, stages 3 and 4 are landed and green.** `npm run test:host -- --nvs <path>` prints
`host: 7 passing, 0 failing`: the isolation claim, and stage 4's six tests in the order the acceptance
check greps for them. The first run on a machine downloads VS Code 1.136.2 (332 MB); later ones reuse
`editors/vscode/.vscode-test/` and take about a minute.

The throwaway profile now starts with **`nvs.lsp.enable` false** (`editors/vscode/scripts/host.mjs:76`),
because "before the server answers" is a state to withhold a server for rather than race; colour's third
test turns it on, which respawns. Mocha loads `*.test.js` in sorted order (`test/host/index.ts:28`), so a
`surfaces.test.ts` runs after `colour.test.ts` with the server already on.

Nothing of stages 5–8 is on disk. Stage 1 is goal `m5-proofs`'s whole list, untouched. CI is not running
(billing block), so stage 6 is proven by reading `ci.yml`, and what that stage still owes beyond the
workflow is a test of `tools/loop.py:1706`'s `EDITOR_READS`, which `tools/` has no suite to host.

## Next group

**Stage 5: the surfaces, in the host** — one file set: `editors/vscode/src/` and
`editors/vscode/test/host/`. The goal's § *Stage 5* names the five `it` titles and their order; the check
greps them, so they are copied exactly rather than paraphrased.

- [ ] **The read-only test surface `activate` returns** (`editors/vscode/src/extension.ts:70`, which
      returns `Promise<void>` today): the status item's current text and severity, the AST view's
      provider, and the ranges last handed to `setDecorations` per editor and decoration kind, collected
      from `editors/vscode/src/redactions.ts:243`, `editors/vscode/src/ast.ts:58` and
      `editors/vscode/src/tasks.ts:57`. No method that changes, reveals or runs anything — the goal's
      § *Standing decisions* is why. It is recorded in `editors/vscode/README.md` § *Decided here*.
- [ ] **The stage-5 fixtures** (new, beside `editors/vscode/test/host/fixture/app.nvs:1`): `broken.nvs`,
      which must fail to compile with an `error[E0301]` the `problemMatcher` can parse, and
      `secrets.nvs`, carrying two `secret` literals so a reveal of the first leaves the second concealed.
      `scripts/host.mjs` copies the whole directory, so neither needs a launcher change.
- [ ] **`test/host/surfaces.test.ts`, the five tests** (new, beside
      `editors/vscode/test/host/colour.test.ts:113`, whose `until` and `open` helpers are the shape to
      copy): the status item's version (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`),
      the Task whose failure reaches `languages.getDiagnostics`
      (`rule:ide/tasks-carry-a-problem-matcher`), the AST panel over a file that does not compile
      (`rule:ide/the-ast-panel-shells-out-to-the-cli`), the two concealed literals and the one reveal
      (`rule:ide/redaction-ranges-come-from-the-server`, `rule:ide/reveal-is-explicit-and-window-local`),
      and no taint decoration at the default setting (`rule:ide/tainted-has-no-default-decoration`).

## Backlog

- Stage 6 owes a test of the acceptance cache's `editors` partition; `tools/` has no suite
  (`docs/agent/loop-goal.md` § *Stage 6*).
- Widening `MEMO_DIRS` in `tools/loop.py` to cover `editors/` — explicitly not this goal
  (`docs/agent/loop-goal.md` § *Standing decisions*).
- The stale trivia paragraph in `docs/plan/m4b.md` is goal `plan-truth`'s.
- The `unowned` module-doc gaps in `crates/nvs-lsp/src/index.rs` and `hints.rs` are goal
  `unowned-closures`'s.
- Marketplace and Open VSX publishing stays open (`rule:ide/one-server-two-thin-clients`
  § *Revisiting*).
