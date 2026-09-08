# Handoff

## State

**Goal 15 stage 6 draws and reveals.** The client conceals every range `nvs lsp` answers for
`nvs/redactions` and uncovers them one at a time. `editors/vscode/src/concealment.ts` holds the
window's state and imports no `vscode`; `src/redactions.ts` is the decoration, the ask and the two
commands' bodies; `nvs.revealSecret` and `nvs.hideSecrets` are registered at
`editors/vscode/src/extension.ts:68`. The headless tier is 155 passing and 0 failing over four
suites — `client:` is the new one, over the pure state machine — and `npm run lint` is clean.

**The concealment outlives the server that answered it.** Only an answer replaces what is held, so
an error, a cancellation, a version refusal or a restart leaves the bar exactly where it was;
`Concealment.hold` is the only writer and `forget`, on the editor closing, the only eraser. That is
`rule:security/redaction-ranges-come-from-the-server`'s fail direction, and `concealment.ts`'s
module doc is where it is written down rather than here.

**Stage 6's third item is server work, not client work.** Nothing the client receives says where a
`tainted` declaration or a sink is; the playbook bullet under *Writing Novis itself* carries the two
anchors and the one route that is open. `nvs.taint.mark` is `off` and draws nothing, which is what
the milestone asks of the default.

## Next group

**Stage 6: where a `tainted` marker's ranges come from** — one file set:
`crates/nvs-lsp/src/redactions.rs`, `crates/nvs-lsp/src/server.rs`, `tests/lsp/redactions/` and
`editors/vscode/src/redactions.ts`. The client half is written and waiting; what is missing is an
answer to draw from, and the request that would carry it already exists.

- [ ] **A second kind on the one request** — a constant beside `SECRET_LITERAL` at
      `crates/nvs-lsp/src/redactions.rs:112`, answered from the walk under it, so a marker's ranges
      arrive on the request that is already there rather than on a new one.
      `rule:ide/the-request-set-is-closed`, and ADR 0101 § 1 is now in this stage's
      `[context] adrs`. Decide there whether the answer stays one list or splits by kind.
- [ ] **An `.lspt` case freezing what it answers** — beside
      `tests/lsp/redactions/a-secret-literal-and-an-interpolation-slot-are-concealed.lspt:1`, over
      the binding shape `tests/lsp/semantic/a-tainted-and-a-secret-binding-carry-their-qualifier-at-every-use.lspt:6`
      writes, so the marker and the concealment are frozen by one corpus.
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
- [ ] **The glyph, only if asked** — `nvs.taint.mark` read where `nvs.secrets.redact` is read at
      `editors/vscode/src/redactions.ts:157`, drawing a themed icon at `declaration` and nothing at
      all at `off`; the colour is a `ThemeColor` and never a literal, the way the concealment bar at
      `editors/vscode/src/redactions.ts:99` already is.
      `rule:security/tainted-has-no-default-decoration`.

## Backlog

- `semanticTokenScopes` for `tainted` and `secret` is still uncontributed, and no rule names the
  scopes to map them to: `rule:ide/novis-ships-names-not-colours` requires the mapping and
  `rule:ide/highlighting-is-two-layers` specifies none. A gap in the rule, and the goal's *Colour is
  specified, not designed* says a session may not close it by choosing.
- Three of the six frozen commands are contributed but unregistered — `nvs.run`, `nvs.test` and
  `nvs.showAst`, all stage 7's.
- Stage 7's Tasks and problem matcher, and stage 8's `.vsix` and CI, are untouched.
- `nvs.lsp.debounce` reaches no server: nothing is sent in `initializationOptions` and the server
  reads none.
- Stage 9's `python tools/dossier.py --only tools:editor/the-vs-code-extension --gate` is the
  earliest red acceptance check and nothing has been written for it yet.
- `docs/agent/loop-goal.toml`'s `[context] modules` names `editors/vscode/src/**`, which matches no
  Rust module.
