# Handoff

## State

**Goal `m4b-editor`, stage 3 (the host harness) is landed and green.** `npm run test:host -- --nvs
<path>` downloads pinned VS Code 1.136.2 into `editors/vscode/.vscode-test/`, opens a copy of
`test/host/fixture/` in a throwaway profile with every other extension disabled, and prints
`host: 1 passing, 0 failing` with exit 0 — the stage-3 acceptance check's three `want` strings.
The first run on a machine downloads 332 MB; later ones reuse the cache and take about a minute.

Stage 2's record and rule are landed. Nothing of stages 4–8 is on disk; every later stage's test is
written into `editors/vscode/test/host/`, which now exists. Stage 1 is goal `m5-proofs`'s whole list,
untouched. CI is not running (billing block), so stage 6 is proven by reading `ci.yml`, and what that
stage still owes beyond the workflow is a test of `tools/loop.py:1706`'s `EDITOR_READS`, which `tools/`
has no suite to host.

## Next group

**Stage 4: colour and activation, in the host** — one file set: `editors/vscode/test/host/`. The
goal's § *Stage 4* names the six `it` titles and their order; the check greps them, so they are copied
exactly rather than paraphrased.

- [ ] **The colour fixtures** (new, beside `editors/vscode/test/host/fixture/hello.nvs:1`): `app.nvs`
      carrying a `secret` binding and a `$total`, `other.php`, and `case.nvst` with a `--FILE--`
      section. `scripts/host.mjs` copies the whole directory, so a file added here is in the workspace
      with no launcher change (`editors/vscode/scripts/host.mjs:70`).
- [ ] **`test/host/colour.test.ts`, the first three tests** (new, beside
      `editors/vscode/test/host/isolation.test.ts:31`): activation on `.nvs` and not `.php`
      (`rule:ide/the-extension-claims-nvs-only`), then `_workbench.captureSyntaxTokens` with
      `nvs.lsp.enable` off and `vscode.provideDocumentSemanticTokens` returning nothing
      (`rule:ide/highlighting-is-two-layers`), then both on with the `secret` modifier present
      (`rule:ide/semantic-tokens-carry-the-qualifiers`). The setting starts from the profile
      `editors/vscode/scripts/host.mjs:78` writes; turning it on mid-suite is
      `getConfiguration().update` at `ConfigurationTarget.Global`, which writes that same file.
- [ ] **The last three tests**: the legend equal to what `initialize` returns, read through
      `editors/vscode/test/protocol/session.ts:77` (the protocol suite compiles to `out/test/protocol/`
      and is importable from the host suite), the `.nvst` case's delimiter and embedded scopes
      (`rule:ide/case-files-have-their-own-grammar`), and `getWordRangeAtPosition` covering the `$` of
      `$total`.

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
