# Handoff

## State

**Goal `template-format` (M10) is met — stage 6, the rulebook, landed with stages 2–5.**
`rule:ide/a-template-region-gets-the-editors-services-and-formatter` is the rule's id everywhere:
the record, the chapter list, the fragment's filename and every citation under `crates/`, `docs/`,
`editors/`, `tests/lsp/` and the website mirror. It is `shipped`, guarded by
`crates/nvs-lsp/tests/regions.rs`, `editors/vscode/test/contributions/contributions.test.ts` and
`editors/vscode/test/surfaces/template.test.ts`, and its formatting paragraph now states what
`editors/vscode/src/format.ts` does rather than what ADR 0173 designed: one chunk at a time, the
four-space unit whatever the editor's own `tabSize` is, and a regions request nothing answers leaving
the markup as `nvs fmt` wrote it. Stage 0's other three sentences were corrected in stages 3–4 and were
re-checked here. Nothing is blocked.

## Next group

**Goal `template-format` has no open stage — these are the two things it deliberately left** — one file
set: `crates/nvs-lsp/src/regions.rs` and `editors/vscode/test/surfaces/template.test.ts`.

- [ ] **A markup literal's body is still not a region** — `crates/nvs-lsp/src/regions.rs:58`
      (§ *What is not a region yet*): `rule:ide/a-template-region-gets-the-editors-services-and-formatter`
      names it one on the same terms as file-scope markup, but the lexer has no token for a segment, so
      the fix is the server's answer and a goal of its own rather than a slice of this one.
- [ ] **The markup pass is only proven against a stub** —
      `editors/vscode/test/surfaces/template.test.ts:202`: VS Code's HTML formatter does not exist
      headless, so every check here injects one. A real template round-trips in M10's host run
      (`docs/plan/m10.md:127`), which no session runs.

## Backlog

- The website mirror was stale before this session; `npm run sync:rules` is human-fired and publishes
  every rule edit since the last one (`.github/workflows/pages.yml`).
- `guardedBy` names no `.lspt` case anywhere in the rulebook, though eight of them cite this rule
  (`docs/rules/ide.json`).
