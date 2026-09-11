# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stage 3 is landed whole.** `contributes` now holds
`taskDefinitions` (one type, `nvs`, requiring `command` and `file`) and one `problemMatchers` entry
named `nvs`, a two-line regex over the renderer's format. `src/tasks.ts` builds the Task both entry
points run, `src/binary.ts` is the one home for which `nvs` is spawned — the server in
`extension.ts` reads it too — and `nvs.run`/`nvs.test` are registered beside the other four
commands. `rule:ide/tasks-carry-a-problem-matcher` is `shipped` and names that suite as its guard.

**The `surfaces:` suite exists**, at `editors/vscode/test/surfaces/`, printing `surfaces: 8
passing`. It runs the contributed matcher over `recorded/check.txt`, which is byte-identical to what
`target/debug/nvs` prints for `recorded/app.nvs` — three diagnostics under two codes, one of them
past line 9 so the widened gutter is covered, and the `aborting due to` summary asserted to start no
entry. Stages 4 to 6 add files to that directory rather than a suite of their own.

**Stage 0 is down to two names, and neither is stage 3's.** `nvs.showAst` is stage 4 and
`nvs.lsp.debounce` is the server half; headless is `192 passing, 2 failing` and
`docs/agent/playbook.md:1268` is what says the `extension` step is red on purpose. Nothing else in
the gate is red.

**`nvs test --list` still has no consumer** — stage 5's explorer is its only one, per
`rule:ide/the-test-tree-is-discovered-and-run-through-the-cli`.

## Next group

**Stage 4: the AST panel** — one file set: `editors/vscode/src/extension.ts`, a new
`editors/vscode/src/ast.ts` beside `tasks.ts`, and `editors/vscode/test/surfaces/`. It answers the
last of stage 0's three unanswered commands, and closes the command half of stage 0's check.

- [ ] **`nvs.showAst` registered, beside the two this session answered**, per
      `rule:ide/the-ast-panel-shells-out-to-the-cli`: the roster's last unanswered id, joining
      `editors/vscode/src/extension.ts:78`. It shells out rather than parsing anything in the
      client, so what it needs from `tasks.ts` is only the binary — `src/binary.ts:15`.
- [ ] **The panel renders `nvs ast --json` for the active file as a tree**, per
      `rule:ide/ast-json-schema-is-frozen`: the document is `kind`, `span`, the node's own scalars
      and `children`, built at `crates/nvs-cli/src/ast.rs:101`, and resilient by default so the
      panel works on a file that does not compile. No second schema and no parser in the client;
      `rule:ide/the-extension-builds-no-ui-the-editor-already-has` puts it in a `TreeView`.
- [ ] **A recorded `nvs ast --json` in the `surfaces` suite**, the way the matcher is recorded:
      a fixture beside `editors/vscode/test/surfaces/recorded/app.nvs`, and an `ast.test.ts` beside
      `editors/vscode/test/surfaces/tasks.test.ts:1` asserting the shape the panel walks. Record it
      from the built binary and diff it back, rather than hand-writing what it should print.

## Backlog

- `nvs.lsp.debounce` is contributed and read by nobody: the client forwards the whole `nvs` section
  already, so the half that is missing is `crates/nvs-lsp/src/settings.rs` reading
  `&["lsp", "debounce"]` and the analysis scheduler honouring it. Goal stage 0, second half.
- Stage 5's Test Explorer over `nvs test --list` and `--format=json` `schemaVersion: 2`.
- Stage 6's `nvs/regions` request and the embedded HTML/CSS/JS services.
- A `.vscodeignore` is still absent, so `vsce package` ships `test/` and its recordings.
