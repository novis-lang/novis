# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stage 2 is landed whole, and the ADR that freezes it is
0172.** `nvs test --format=json` is `schemaVersion: 2` with `file`, `line` and `column` on every
record; `nvs test --list` writes the discovery document, whose records sit under `listed` and carry
no verdict; `nvs check --json` is version 1 and unchanged. The three tests stage 2's second
`[[check]]` names pass in `crates/nvs-cli/src/runner.rs`, and two conformance cases pin both
documents. JUnit and the plaintext report are byte-identical to what they were.

**Stage 0 is where it was, and stages 3 and 4 are what close it.** `tools/verify.py`'s `extension`
step and the goal's earliest acceptance check are the same two contributions tests — `nvs.run`,
`nvs.test` and `nvs.showAst` are contributed and unanswered — and `docs/agent/playbook.md:1268` is
what says so. Nothing else in the gate is red.

**`nvs test --list` is on the CLI and nothing reads it yet.** Stage 5's explorer is its only
consumer, and its shape is `rule:ide/the-test-tree-is-discovered-and-run-through-the-cli`.

## Next group

**Stage 3: the Tasks, and the Problems panel** — one file set: `editors/vscode/package.json`,
`editors/vscode/src/extension.ts`, `editors/vscode/scripts/headless.mjs` and a new
`editors/vscode/test/surfaces/`. The first two answer two of stage 0's three unanswered commands;
the third is the suite the stage's own `[[check]]` greps for as `surfaces:`.

- [ ] **`nvs run` and `nvs test` contributed as Tasks, each with a `problemMatcher`**, per
      `rule:ide/tasks-carry-a-problem-matcher`: `contributes` at `editors/vscode/package.json:28`
      holds neither `taskDefinitions` nor `problemMatchers` today. The pattern is two lines over the
      renderer's existing format — `error[E0301]: message`, then `  --> file:line:col`
      (`rule:errors/renderings`) — and nothing in the client parses a diagnostic itself.
- [ ] **`nvs.run` and `nvs.test` answered as the commands that start those Tasks**, per
      `rule:ide/contributions-are-frozen-and-only-ever-added`: register them beside the three at
      `editors/vscode/src/extension.ts:72`, and let them execute the contributed Task rather than
      spawning a second process. The roster the contributions test asserts is
      `editors/vscode/test/contributions/contributions.test.ts:96`.
- [ ] **The `surfaces:` suite exists and is discovered**, per the stage's own check: `ORDER` at
      `editors/vscode/scripts/headless.mjs:21` lists three suites and the check wants four. Its
      first case is the `problemMatcher`'s regex against a recorded `nvs check` rendering carrying a
      real `error[E0301]` and its `-->` line — a regex over recorded text, so no editor runs.

## Backlog

- Stage 4: `nvs.showAst`, per `rule:ide/the-ast-panel-shells-out-to-the-cli` — the last of stage 0's
  three unanswered commands, and the AST panel's whole client half.
- Stage 5: the `TestController`, per `rule:ide/the-test-tree-is-discovered-and-run-through-the-cli`
  — discovery from `--list`, runs through `--format=json --filter`, and `.nvst` as a second suite.
- Stage 6: `nvs/regions` and the template services, per
  `rule:ide/a-template-region-gets-services-but-no-second-formatter`.
- `nvs test --coverage` exports nothing, so the explorer ships without coverage; `docs/plan/m10.md`
  owns when the Clover/lcov exporters land.
