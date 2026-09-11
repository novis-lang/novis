# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stages 0 and 5 are closed.** The Test Explorer is built:
`editors/vscode` headless is **209 passing, 0 failing**, and the `surfaces:` suite is 23 of those.
The stage 5 check at `docs/agent/loop-goal.toml:7903` names its three cases literally and they now
exist, so that check is green rather than green-by-label.

**The explorer is one `TestController` holding two suites.** `editors/vscode/src/tests.ts` is the
controller, the processes and the editor's run object; `editors/vscode/src/report.ts` is what the two
documents *mean* and imports no `vscode`, which is where the headless tier tests it — the same split
`ast.ts`/`nodes.ts` already uses. Discovery is `nvs test --list --format=json` per program, lazily,
so nothing runs while the tree is built. A run is `nvs test --format=json` per file, mapped back onto
the queued items by `Class::method`. The `.nvst` half is one process per case file and the exit
status is the verdict, because `nvs test` refuses `--format` over a case tree
(`crates/nvs-cli/src/main.rs:2105`) and nothing here parses a human rendering to get around that.
`flaky` reads as failed and `exited` as errored — the editor has four states where the runner has
five, and `rule:testing/test-attribute` § 20 says which way that resolves.

**`recorded/suite.nvs`, `recorded/list.json` and `recorded/report.json`** are a two-class program and
the two documents the binary printed for it, byte for byte. Re-record both after any edit to it.

**The chain check was red on a dangling selector, not on the tree.** The goal named a playbook bullet
`Tooling > editors/vscode/package.json's contributes` that no commit ever wrote; it is gone and
`python tools/chain.py --check` passes.

## Next group

**Stage 6: the template regions** — one file set: `crates/nvs-lsp/` for the request and its two guard
tests, then `editors/vscode/` for the client half. Nothing of `nvs/regions` exists on either side
yet, so the two checks at `docs/agent/loop-goal.toml:7929` and `:7942` are red on unbuilt work rather
than on a regression. `nvs/redactions` is the shape to copy at every anchor below — it is the only
other request of Novis's own.

- [ ] **`nvs/regions` answers the lexer's mode boundaries**, per
      `rule:ide/a-template-region-gets-services-but-no-second-formatter`: a `METHOD` const beside
      `crates/nvs-lsp/src/redactions.rs:137`, a handler beside `crates/nvs-lsp/src/server.rs:1333`,
      an `Answer` arm at `crates/nvs-lsp/src/render.rs:190`, and the guard test
      `a_region_answer_carries_a_span_and_a_language_and_nothing_else` the check names. The other
      named test, `the_server_declares_no_formatting_provider`, is about
      `crates/nvs-lsp/src/capabilities.rs:285`, which already declares none — it pins that.
- [ ] **The answer is frozen as a `.lspt` case like every other one**, per
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`: a `Request` variant beside
      `crates/nvs-lsp/src/case.rs:93`, its arm beside `crates/nvs-lsp/src/suite.rs:548`, and a case
      under `tests/lsp/`. The check is `kind = "nvs-suite"` over that tree.
- [ ] **The client forwards inside a region and nowhere else**, per the same rule: a new
      `editors/vscode/src/regions.ts` on `editors/vscode/src/redactions.ts:51`'s shape, installed
      beside `editors/vscode/src/extension.ts:105`, the contributed setting `nvs.template.services`
      added to the manifest's roster, and `editors/vscode/test/surfaces/regions.test.ts` whose two
      case titles the check at `docs/agent/loop-goal.toml:7942` matches literally.

## Backlog

- Coverage in the explorer waits for the Clover/lcov exporters; refused as a stub, per the goal's
  § *Standing decisions*.
- The reference chapter check (`python tools/reference.py --check`) is stage 6's last item.
- Discovery compiles every `.nvs` in the workspace when the Testing view opens; if that is measured
  as too slow, the cost is `editors/vscode/src/tests.ts`'s `discover`.
- `docs/agent/carried-gaps.md` is where anything above goes if this goal is switched.
