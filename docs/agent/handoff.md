# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Stages 0 and 5 are closed; stage 6 is two thirds done.**
`nvs/regions` is answered, frozen as `.lspt` cases and guarded: `cargo test -p nvs-lsp` and
`nvs lsp-test tests/lsp/` (267 passing) are both green, so the two stage-6 checks at
`docs/agent/loop-goal.toml:7915` and `:7921` are closed. What is left of the stage is the client half
and the `editors/vscode` headless check at `docs/agent/loop-goal.toml:7935`.

**The request is a lex, not an analysis.** `crates/nvs-lsp/src/regions.rs` maps every
`TokenKind::InlineHtml` run to a `{range, language}`, and the server handler lexes the open buffer
rather than resolving a graph — a boundary carries no type question and the client re-asks it on every
edit. The `.lspt` suite calls `for_document` over the analysis it already holds; both reach the same
walk. A markup literal's body (``html`…` ``) is named by the rule and is not answered, because the
lexer has no token for one yet; that is written down in the module doc, not here.

**Completion now answers nothing inside a run of markup** (`crates/nvs-lsp/src/completion.rs:321`).
It offered the whole keyword list, which inserts text a page renders rather than runs; the region is
the HTML service's, which is what separates markup from the other three `NOT_CODE` spellings.

## Next group

**Stage 6: the client half** — one file set: `editors/vscode/`, and `editors/vscode/src/redactions.ts`
is the shape to copy at every anchor below, being the only other client of a request of Novis's own.

- [ ] **`nvs.template.services` joins the frozen roster**, per
      `rule:ide/contributions-are-frozen-and-only-ever-added`: a `boolean` property defaulting to
      `true` beside `editors/vscode/package.json:128`, and the two rows the contributions test asserts
      it by — the name in the sorted list at
      `editors/vscode/test/contributions/contributions.test.ts:88` and the default at
      `editors/vscode/test/contributions/contributions.test.ts:371`.
- [ ] **The client forwards inside a region and nowhere else**, per
      `rule:ide/a-template-region-gets-services-but-no-second-formatter`: a `editors/vscode/src/template.ts`
      modelled on `editors/vscode/src/redactions.ts:51` (the `METHOD` const), `:129` (`serve`) and
      `:216` (the `sendRequest` and its cached answer), started beside the client at
      `editors/vscode/src/extension.ts:149`. It registers no formatting provider — that is the half
      the rule turns on, and `crates/nvs-lsp/src/capabilities.rs:287` is the server's end of it.
- [ ] **The headless tier holds both**, per the check's own `want` list at
      `docs/agent/loop-goal.toml:7935`: a `editors/vscode/test/surfaces/template.test.ts` whose
      `describe` is `the template regions` and whose two cases are spelled
      `forwards inside a region and nothing outside one` and
      `forwards nothing at all when nvs.template.services is false` — the `want` list is ordered, so
      those strings are the specification and not a paraphrase of it.

## Backlog

- A markup literal's body is a region the lexer cannot yet report — `crates/nvs-lsp/src/regions.rs`'s
  module doc, § *What is not a region yet*.
- `nvs.completion.phpNames` and `nvs.checkWorkspace` are on the frozen roster and not yet contributed
  — `rule:ide/contributions-are-frozen-and-only-ever-added`.
