# Handoff

## State

**Goal `m4b-editor` is met: stages 0 through 8 are on disk and every one of its checks passes
locally.** This session closed the one check the driver reported red after session 0006.

- **That check was never a download failure.** The ledger quoted `- Resolving version...`, which is
  `@vscode/test-electron` announcing the cache hit it went on to use; the run's own console log has
  `conceals both secrets on open and reveals exactly one: ... did not happen within 20000ms` and
  `host: 11 passing, 1 failing` under the same check.
- **Its cause is established and closed.** `redactions.ts` asked `nvs/redactions` from a
  `workspace.onDidOpenTextDocument` listener installed at activation, and VS Code calls that before
  the listener `vscode-languageclient` registers when it starts — so the ask could reach the server
  ahead of the notification that opens the document there. `crates/nvs-lsp/src/document.rs:382`
  answers `None` for a document nothing is open for, `crates/nvs-lsp/src/server.rs:1357` turns that
  into `[]`, and `editors/vscode/src/concealment.ts:153` counts an empty answer as an answer, so
  nothing re-asks: the `secret` stays on screen in cleartext until the file is edited. The ask is
  the client's own `didOpen` middleware now (`editors/vscode/src/extension.ts:190`), which awaits
  the notification before it asks.
- **Evidence.** The host suite ran five times at head, `host: 12 passing, 0 failing` every time,
  the last of them with the fix in place. `npm run lint` is clean.

The pack still never prints the goal's own `## Stage N` prose, where each stage's exact markers
are, and `[context]` has no field that reaches it. One `peek.py` target on
`docs/agent/loop-goal.md:"## Stage N"` is the whole fix, but it has to be remembered.

## Next group

**Nothing of this goal is open** — the status line is `DONE` and the next session is the chain's
next goal, which `python tools/brief.py` names after `tools/goal-switch.py` runs. If the driver's
sweep declines the claim again, the work is whichever check it names, and only these two could be
it:

- [ ] **A red stage 3–5 host check: read the console log, never the ledger's quoted line**
      (`docs/agent/loop-goal.toml:9915` is the isolation check; `.loop/logs/<run>-console.log` holds
      the whole output, and `editors/vscode/test/host/surfaces.test.ts:47`'s `until` now says what
      was drawn instead of what was waited for). One `npm run test:host -- --nvs
      target/debug/nvs.exe` from `editors/vscode` reproduces all twelve.
- [ ] **A red stage 8 check means the render is stale**: `python tools/rules.py --render` after any
      edit under `docs/rules/` (`docs/rules/ide.json:1` is the chapter this goal ships three rules
      into).

## Backlog

- `tools/loop.py:1368` — `first_err_line`'s `startswith("error")` is case-sensitive, so a Mocha
  `Error: host: 1 failing` loses to stderr's first line and the ledger quotes progress output.
- A test of `tools/loop.py:1706`'s `EDITOR_READS`: `tools/` has no suite to host one
  (`docs/agent/loop-goal.md` § *The acceptance cache sees an extension change*).
- Publishing the `.vsix` to the Marketplace or Open VSX, or attaching it to a release —
  `rule:ide/one-server-two-thin-clients` § *Revisiting* keeps it open.
- A pixel tier for the colour assertions; PhpStorm and every M10 editor feature — not this goal.
- The stale trivia paragraph in `docs/plan/m4b.md`, which is goal `plan-truth`'s.
- The `unowned` module-doc gaps in `crates/nvs-lsp/src/index.rs` and `hints.rs`, goal
  `unowned-closures`'s.
