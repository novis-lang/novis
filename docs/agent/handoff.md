# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stage 4 is landed whole.** `nvs.showAst` is registered
beside the other five commands, `editors/vscode/src/ast.ts` is the panel — it shells out to
`nvs ast --json`, reads standard output whatever the exit status was, and hangs the document in a
`TreeDataProvider` — and `editors/vscode/src/nodes.ts` is the vscode-free half that decides what a
node reads as. The manifest contributes one `views.explorer` entry, `nvs.ast`, collapsed, with a
`viewsWelcome` that runs the command.

**Stage 0's command half is closed.** Every contributed command now reaches a `registerCommand`,
and headless is `205 passing, 1 failing` — the one is `nvs.lsp.debounce`, contributed and nothing
reads it, which is the next group below. `docs/agent/playbook.md:1268` is still what says the
`extension` step of the gate is red on purpose.

**The `surfaces:` suite is 20 passing.** `recorded/ast.json` is byte-identical to what
`target/debug/nvs` prints for `recorded/app.nvs`, which now ends in `echo 1 + 2;` and `echo true;`
so the recording carries both shapes of scalar field the schema has — a word and a flag.
`check.txt` is unchanged by that edit and was re-recorded to prove it, so stage 3's assertions
still stand on the same program.

**A span is bytes; every editor position is UTF-16 code units.** `nodes.ts`'s `index()` converts,
and the suite pins it over an accented letter and an astral character. Anything else in this
client that turns a compiler offset into a `Position` owes the same conversion.

**`nvs test --list` still has no consumer** — stage 5's explorer is its only one, per
`rule:ide/the-test-tree-is-discovered-and-run-through-the-cli`.

## Next group

**Stage 0: the last unanswered setting** — one file set: `crates/nvs-lsp/src/settings.rs` and
`crates/nvs-lsp/src/server.rs`. It closes the check the driver is red on, whose other half landed
this session; `editors/vscode/test/contributions/contributions.test.ts` is the gate and needs no
edit.

- [ ] **`Settings` learns `nvs.lsp.debounce`**, per
      `rule:ide/contributions-are-frozen-and-only-ever-added`: a contributed setting is answered,
      never removed. It joins the three reads at `crates/nvs-lsp/src/settings.rs:113`, and the
      contributions suite matches the literal `&["lsp", "debounce"]`, so the path is spelled that
      way or the gate stays red however the value is read.
- [ ] **The change notification defers re-analysis by it**, at `crates/nvs-lsp/src/server.rs:1290`,
      which is `DidChangeTextDocument`'s arm. No rule specifies what the interval *means* —
      `python tools/brief.py --where debounce` finds none — so take the roster's own sentence and
      that module doc's "a setting reaches this server exactly once": hold the analysis for that
      many milliseconds after the last change, publish once, and treat `0` as no wait.

## Backlog

- Stage 5, the Test Explorer, and the only consumer `nvs test --list` has — `docs/agent/loop-goal.md` § *Stage 5*.
- Stage 6, the inline-HTML region and `nvs/regions` — `rule:ide/a-template-region-gets-services-but-no-second-formatter`.
- The AST panel re-reads on save of the file it is showing and on nothing else — `editors/vscode/src/ast.ts`.
- `[context] modules` does not name `crates/nvs-lsp/src/server.rs`, which the next group edits; the pack printed only `settings.rs` for that crate.
- `docs/decisions/` still owes this goal's one ADR, for stage 2's schemas and stage 5's explorer shape — `docs/agent/loop-goal.md` § *Standing decisions*.
