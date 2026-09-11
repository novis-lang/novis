# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stage 0 is closed whole.** Every contributed command
reaches a `registerCommand` and every contributed setting is read: `nvs.lsp.debounce` was the last
unanswered one, and `editors/vscode` headless is **206 passing, 0 failing**. The vscode leg of the
gate is `python tools/playbook.py --show 'Tooling > tools/verify.py runs'`.

**`nvs.lsp.debounce` is read and it does something.** `Settings` carries `debounce: Duration`
(`crates/nvs-lsp/src/settings.rs:70`), defaulting to the same 150 ms the manifest declares;
`serve`'s loop holds one deferred `Changed` with a deadline and waits on the channel with
`recv_timeout`, and `reanalyse` is the index refresh plus the publish it defers. A keystroke in the
same buffer replaces the one waiting and that analysis never runs; an open, a close, an edit to
another buffer, or **any request** flushes it first, because an answer is about the buffer as it
stands. Zero is the setting turned off. `crates/nvs-lsp/tests/publish.rs` pins the replacement with a
window no case waits out, so the case has no clock in it.

**Stages 5 and 6's client half are unbuilt, and nothing was red about that until this session.**
Stage 3's check greps for the `surfaces:` label, which `ast.test.ts` and `tasks.test.ts` produce on
their own, so every check in this goal passed with the Test Explorer and the region forwarding
missing. Two new checks now name their cases — `docs/agent/loop-goal.toml:7903` and `:7942`. **Both
are red on purpose and neither is a regression**; the playbook bullet this session added is the
general form.

**The `surfaces:` suite is 20 passing**, over the AST panel and the Tasks only.
`recorded/ast.json` is byte-identical to what `target/debug/nvs` prints for `recorded/app.nvs`.

## Next group

**Stage 5: the Test Explorer** — one file set: `editors/vscode/src/extension.ts`, a new
`editors/vscode/src/tests.ts`, and `editors/vscode/test/surfaces/tests.test.ts` with its recordings
under `editors/vscode/test/surfaces/recorded/`. The case titles are not free: the check at
`docs/agent/loop-goal.toml:7903` matches them literally.

- [ ] **The controller is populated from the listing mode**, per
      `rule:ide/the-extension-builds-no-ui-the-editor-already-has`: a `TestController` created beside
      the six commands at `editors/vscode/src/extension.ts:83`, filled from `nvs test --list`, whose
      document is `listing_document` at `crates/nvs-cli/src/runner.rs:1841`. Nothing runs during
      discovery. The case is titled `populates the tree from the listing before anything runs`.
- [ ] **A run goes through the json report**, per the same rule: `nvs test --format json`'s document
      is `json_document` at `crates/nvs-cli/src/runner.rs:1734`, and each record's verdict and
      failures become the item's state and message — no second parse of the human rendering. The case
      is titled `runs one case and reads its verdict and failure back`.
- [ ] **The `.nvst` corpus is a second suite in the same controller**, per
      `rule:ide/lspt-coverage-is-inferred`: two rosters, one explorer, so the corpus joins through the
      same listing seam at `crates/nvs-cli/src/runner.rs:1813` rather than a directory walk in the
      client. The case is titled `holds the .nvst corpus as a second suite in the same controller`.

## Backlog

- Stage 6's client half — forwarding into a region behind `nvs.template.services`, gated by
  `docs/agent/loop-goal.toml:7942` and specified by
  `rule:ide/a-template-region-gets-services-but-no-second-formatter`.
- No coverage, deliberately: goal § *Standing decisions* refuses wiring `FileCoverage` to exporters
  that do not exist.
- Four `# TODO: why a session opens this.` lines in `docs/agent/loop-goal.toml`'s `[context] modules`,
  swept in by the driver and never annotated.
- `docs/reference/tools/40-editor.md` § *What it does not do* still has to stop naming what stages 5
  and 6 answer; the `reference.py --check` leg is what notices.
