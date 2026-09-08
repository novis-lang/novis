# Handoff

## State

**Goal 15 stage 9 is done, and it was the last stage red.** `docs/reference/tools/40-editor.md`
carries a third `#` heading — *The VS Code extension* — beside `# nvs lsp` and `# nvs lsp-test`, and
`python tools/dossier.py --group tools:editor` shows all three features complete. Both stage-9
checks are green (`dossier.py --only tools:editor/the-vs-code-extension --gate`, `reference.py
--check`), and `verify.py` is 8 of 8: 3574 Rust, 1592 conformance, 276 differential, 102 headless.

**Two contributions are documented as not answered, because they are not.** `nvs.run`, `nvs.test`
and `nvs.showAst` are declared in `editors/vscode/package.json:119` and registered by nothing
(`editors/vscode/src/extension.ts:62` registers three others), and `nvs.lsp.debounce` is read by no
client and no server. The chapter's § *What it does not do* says so rather than describing them as
working; `playbook.md` § *Tooling* has the trap.

**The chapter and the manifest are now pinned to each other.**
`crates/nvs-lsp/tests/extension_reference.rs` reads both and fails when the settings table, the
commands table or the `.nvs`-only claim drifts from what the editor actually reads. The extension's
own headless suite freezes the manifest against a TypeScript roster and never opens the chapter,
which is the half that was unchecked.

## Next group

**The extension's unanswered contributions** — one file set: `editors/vscode/package.json`,
`editors/vscode/src/extension.ts`, `docs/reference/tools/40-editor.md`. Each item closes one of the
"contributed and not yet answered" rows the chapter now names, and the last line of that section is
what each one deletes.

- [ ] **`nvs.run` and `nvs.test` become Tasks and a command** — `rule:ide/vscode-is-the-reference-client`
      names both as VS Code Tasks plus a "Run File" command. The registration goes beside the three
      at `editors/vscode/src/extension.ts:62`; the identifiers are already frozen at
      `editors/vscode/package.json:119`. A task's failures must populate the Problems panel, which
      is `docs/plan/m4b.md`'s acceptance sentence.
- [ ] **`nvs.showAst` renders the panel** — `rule:ide/the-extension-builds-no-ui-the-editor-already-has`
      is the bound on what may be built for it, and `docs/plan/m4b.md`'s acceptance asks for a file
      that does not compile to still render. Registration at `editors/vscode/src/extension.ts:62`,
      identifier at `editors/vscode/package.json:131`.
- [ ] **`nvs.lsp.debounce` reaches something** — the client builds `LanguageClientOptions` with no
      `initializationOptions` at `editors/vscode/src/extension.ts:103`, and the server has no
      debounce at all (`crates/nvs-lsp/src/server.rs`). Either it is wired end to end or the setting
      is withdrawn; `rule:ide/one-server-two-thin-clients` decides which side holds the timer.

## Backlog

- No `semanticTokenScopes` contribution, so the two Novis modifiers fall back to the underlying
  token type — `rule:ide/novis-ships-names-not-colours` asks for the mapping.
- Formatting is in `rule:ide/vscode-is-the-reference-client` and in no capability the server
  declares; `nvs-fmt` is not wired to the client.
- `nvs.taint.mark = sink` marks what `declaration` marks —
  `crates/nvs-lsp/src/redactions.rs` § *What it does not reach*.
- The extension-host tier stays CI's, on Linux under `xvfb-run`
  (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`).
