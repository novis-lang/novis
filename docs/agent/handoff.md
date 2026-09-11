# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stage 0 is half closed and stage 2 is untouched.**
`editors/vscode/test/contributions/contributions.test.ts` now asserts both answering claims — every
contributed command reaches a `registerCommand` call, every contributed setting reaches a reader —
and it fails on exactly four names: `nvs.run`, `nvs.test`, `nvs.showAst` (stages 3 and 4) and
`nvs.lsp.debounce` (nothing reads it on either side of the wire).

**`tools/verify.py`'s `extension` step is therefore red on purpose**, two failing tests out of 186,
and the playbook bullet at `docs/agent/playbook.md:1268` is what says so. Nothing else in the gate
is red. Do not widen either assertion and do not remove an id
(`rule:ide/contributions-are-frozen-and-only-ever-added`).

The settings half was one name wider than the item claimed: the client passed no
`initializationOptions` at all, so `nvs.completion.phpNames` was inert too although
`crates/nvs-lsp/src/settings.rs:110` has read it since goal `workspace-index`. The client now hands
over the `nvs` section whole, which also made `nvs.check.scope` and `nvs.codeLens.enable` — read by
that same server, contributed by nobody — worth contributing, so they are on the roster now.

## Next group

**Stage 2: the two CLI surfaces, and the one ADR this goal opens** — one file set: `crates/nvs-cli/`
and `crates/nvs-diagnostics/`. Both are data the compiler already holds, rendered a second way, and
two later client stages query them, so they go before any more TypeScript.

- [ ] **`nvs check --json`**, per `rule:ide/check-json-is-the-diagnostic-record-as-a-document`: one
      record per diagnostic the text renderer prints — code, span, severity, help, `suggestions` —
      under a frozen schema, with the text rendering still the default. The flag goes on
      `Command::Check` at `crates/nvs-cli/src/main.rs:895`, the record it writes is
      `crates/nvs-diagnostics/src/diagnostic.rs:128`, and `crates/nvs-cli/src/ast.rs:54` is the
      frozen `--json` renderer to model it on rather than invent a second convention. Its three test
      names are in the stage 2 `[[check]]` block.
- [ ] **The test report to `schemaVersion: 2`**, per the same stage: each record gains its declaring
      file and line, and a listing mode discovers without executing. `json_document` at
      `crates/nvs-cli/src/runner.rs:1653` is the document; JUnit and the human format stay
      byte-identical.
- [ ] **One ADR** covering both schemas above and stage 5's explorer shape — the only record this
      goal opens. Re-derive the next free number from `docs/decisions/` immediately before creating
      it; it was 0172 at this commit. The shape, and the `changes:`/`because` relation it must write
      twice, is `docs/agent/conventions.md:443`.

## Backlog

- `nvs.lsp.debounce` is stage 0's remaining half: nothing in `nvs-lsp` debounces anything, and
  `Settings::from_initialize` (`crates/nvs-lsp/src/settings.rs:110`) is where it would be read.
- `nvs.checkWorkspace` and `nvs.template.services` are in `rule:ide/contributions-are-frozen-and-only-ever-added`'s
  M10 roster and still contributed by nobody; each arrives with its feature, not before it.
- Stages 3 and 4 own the three unanswered commands; stage 5 the explorer, stage 6 the regions.
- `docs/agent/loop-goal.toml`'s `[context]` gained `ide/check-scope-defaults-to-open-documents` and
  three modules (`ast.rs`, `diagnostic.rs`'s crate already listed, `nvs-lsp/src/settings.rs`) this
  session; `docs/agent/goals/39-editor-surfaces.toml` is the byte-identical copy.
