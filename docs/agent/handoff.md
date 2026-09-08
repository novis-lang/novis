# Handoff

## State

**Goal 15 stage 5 is whole, and the acceptance check `vscode (headless)` is green** — `grammar:`,
`contributions:` and `protocol:` all print, at 141 passing and 0 failing, and `npm run lint` is
clean. `editors/vscode/src/` exists: `extension.ts` spawns `nvs lsp` from `nvs.path` falling back to
`PATH` and reports health through a `LanguageStatusItem`, and `version.ts` holds the series
comparison and imports no `vscode`, which is what lets the headless tier check it.

**The extension is versioned `0.0.1`, the workspace's own**, and the client understands a server in
its own `major.minor`. That decision and the two it replaced are in
`editors/vscode/README.md` § *Decided here*, not here.

**`nvs lsp` used to outlive every session it started**: `run()` joined the io threads while the
connection still held a sender, so the writer thread never ended. Fixed at
`crates/nvs-lsp/src/server.rs:87`, and `handshake.test.ts`'s last case is what would catch it again
— nothing that speaks the protocol without owning the process can see it.

**Stage 6 is next and stage 8 is unblocked**: `main` is in the manifest, so a `.vsix` is now worth
building.

## Next group

**Stage 6: `secret` concealed, `tainted` left alone** — one file set: `editors/vscode/src/`,
`editors/vscode/package.json` and `editors/vscode/test/protocol/`. The server half landed with goal
14, so this group draws what it is handed and decides nothing.

- [ ] **The redactions the client draws** — a new `editors/vscode/src/redactions.ts`, wired in
      beside the client at `editors/vscode/src/extension.ts:110`, asking the server's own request
      (`nvs/redactions`, `crates/nvs-lsp/src/redactions.rs:106`) and concealing each range with a
      `TextEditorDecorationType` that keeps the character cells, so every edit still addresses the
      real text. Default on, behind `nvs.secrets.redact`.
      `rule:security/redaction-ranges-come-from-the-server`.
- [ ] **The reveal state machine** — `nvs.revealSecret` and `nvs.hideSecrets` registered beside
      `nvs.restartServer` at `editors/vscode/src/extension.ts:59`, revealing **one** range and
      leaving a second secret in the same file hidden, and forgetting every reveal when the editor
      closes. Keep the decision in a module that imports no `vscode`, the way
      `editors/vscode/src/version.ts:1` does, so the headless tier can hold its test.
- [ ] **`tainted` decorated only if asked** — `nvs.taint.mark` is `off` by default and draws
      nothing; `declaration` and `sink` are the two glyph placements the setting's
      `enumDescriptions` at `editors/vscode/package.json:109` already promise.
      `rule:security/tainted-has-no-default-decoration`.

## Backlog

- `semanticTokenScopes` for `tainted` and `secret` is not contributed yet; without it a theme has
  nothing to match — `rule:ide/novis-ships-names-not-colours`.
- Five of the six frozen commands are contributed but unregistered, so the palette offers them and
  they fail — stages 6 and 7 register them.
- `nvs.lsp.debounce` reaches no server: nothing is sent in `initializationOptions` and the server
  reads none.
- Stage 7's Tasks and problem matcher, and stage 8's `.vsix` and CI, are untouched.
- `docs/agent/loop-goal.toml`'s `[context] modules` names `editors/vscode/src/**`, which matches no
  Rust module and warns every session; the pack has no field for a TypeScript directory.
