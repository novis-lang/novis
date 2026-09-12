# Handoff

## State

**Goal `template-format` (M10) — stages 4 and 5 have landed; only stage 6, the rulebook, is left.**
The markup half is `editors/vscode/src/template.ts`: `chunks` builds them from the formatted text, the
server's regions and the text's line breaks alone, `hidden` shows a formatter a stand-in of each hole's
own length, `merged` re-bases an answer onto the chunk's base and drops whole a chunk whose stand-ins
did not survive, and `edited` is the one conversion from a formatter's edits to a text.
`editors/vscode/src/format.ts` runs ADR 0173 § 1's order — `nvs fmt`, `nvs/regions` over its answer,
the editor's `html` **range** formatter over one virtual document per chunk at `{ tabSize: 4,
insertSpaces: true }` — and `editors/vscode/src/regions.ts` now exports `regions(document, text?)` and
`embedding(document, language, as, text)` for it. `nvs.template.format` is contributed, default `true`,
read where the file is, and in `docs/reference/tools/40-editor.md`'s table, which
`crates/nvs-lsp/tests/extension_reference.rs` holds to the manifest. The headless suite is 222 passing
and carries the stage-4 check's titles in its order. Nothing is blocked.

## Next group

**Stage 6: the rulebook — the id, the status, the guards** — one file set: `docs/rules/ide.json`, the
fragment beside it, and the 32 files that cite the old id.

- [ ] **The rename** — `docs/rules/ide.json:596`: the id becomes
      `ide/a-template-region-gets-the-editors-services-and-formatter`, in the record, in the chapter's
      own list (`docs/rules/ide.json:246`), in the fragment's filename, and in every `rule:` citation
      of it under `docs/`, `crates/`, `editors/` and `tools/` — 32 files — then `python tools/rules.py
      --render`. The goal's § *Stage 6* is the item; the check is `rules.py --show <new id>`.
- [ ] **Shipped, with its guards** — `docs/rules/ide.json:596`: status `shipped`, `guardedBy` filled
      from this goal's tests and goal `editor-surfaces`'s —
      `editors/vscode/test/surfaces/template.test.ts:202` (the format cases) and
      `editors/vscode/test/contributions/contributions.test.ts:83` (the frozen roster).
- [ ] **The sentences stages 3–5 settled** —
      `docs/rules/ide/a-template-region-gets-services-but-no-second-formatter.md:18`: the formatting
      paragraph is now shipped behaviour, so read it against `editors/vscode/src/format.ts:106` and the
      two settings' names against `editors/vscode/package.json:133`, and correct what it says rather
      than adding to it.

## Backlog

- A hole written across lines becomes one long line of stand-in, so a formatter that wraps it leaves
  that chunk as written — `editors/vscode/src/template.ts`'s `hidden` owns that trade.
- A real HTML formatter over a real template is the milestone's host run, never headless —
  `docs/plan/m10.md` § *Verify*.
- A markup literal's body is still not a region, so `nvs fmt` alone lays one out —
  `crates/nvs-lsp/src/regions.rs` § *What is not a region yet*.
