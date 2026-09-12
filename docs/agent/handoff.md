# Handoff

## State

**Goal `template-format` (M10) — stage 3 has landed: format-on-save starts `nvs fmt`.**
`editors/vscode/src/format.ts` registers the client's one `DocumentFormattingEditProvider` for the
`nvs` selector, runs `nvs fmt --stdin` through `binary()` and answers one whole-document edit; it is
installed from `activate` beside `regions.install`. What the edit is — nothing on a refusal, nothing
on an already-canonical buffer, otherwise the formatter's text whole — is `formatted`
(`editors/vscode/src/template.ts:136`), the half that imports no `vscode` and runs headless.

Stage 0's three prose corrections are done (`editors/vscode/src/regions.ts:10`,
`crates/nvs-lsp/src/regions.rs:66`, and the suite's registration assertion, narrowed to *one
provider, in `format.ts`*). Stage 0's fourth item, the rule id rename, is stage 6's.

Nothing of stages 4–6 exists: `template.ts` has no chunking, `format.ts` asks `nvs/regions` nothing,
and `nvs.template.format` is not in the manifest. **The stage-4 check's `want` list interleaves the
two remaining stages** — its `formats only the Novis half when nvs.template.format is false` names
stage 5's setting, and the case for it can be written at stage 4 by passing `false` to the pure
decision, the way `forwarded(enabled, …)` already takes the services setting. Nothing is blocked;
the design is ADR 0173's and is not re-opened.

## Next group

**Stage 4: the markup half — chunks, the base, and the holes** — one file set:
`editors/vscode/src/template.ts`, `editors/vscode/src/format.ts`, and the suite beside them.

- [ ] **The chunks and the base** — `editors/vscode/src/template.ts:107` (beside `virtual`): the
      markup between a `?>` that ends its line and the `<?nvs` that reopens code, holes included,
      built from the formatted text and the server's regions and from nothing else; the base is the
      indentation of the `?>` line, column zero before a file's first open tag, and the closing
      `<?nvs` line sits at the base too (ADR 0173 §§ 2–3;
      `rule:ide/a-template-region-gets-services-but-no-second-formatter`).
- [ ] **The stand-in and the merge** — `editors/vscode/src/template.ts:136` (beside `formatted`,
      which grows the markup half of its decision): a hole is shown to the formatter as a stand-in
      of its own length so every position maps back, the output is re-based onto the chunk's base
      whatever column the formatter started at, and a chunk whose edit would reach a hole is left as
      written (ADR 0173 § 2; the goal's § *Standing decisions*, *Refuse, never guess*).
- [ ] **The pass** — `editors/vscode/src/format.ts:63`: after `nvs fmt`, ask `nvs/regions` with the
      formatted text (the client is `editors/vscode/src/regions.ts:278`'s shape, plus `text`), run
      the editor's `html` **range** formatter over a virtual document per chunk with
      `{ tabSize: 4, insertSpaces: true }`, and hand the results to the pure half. A regions request
      that fails leaves the markup as `nvs fmt` wrote it.
- [ ] **The cases** — `editors/vscode/test/surfaces/template.test.ts:119`, under the existing
      `describe("the template format")` and after its two cases: the stage-4 check's remaining
      titles in its order, with the HTML formatter injected as a stub
      (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). The floor pins
      `the template regions` and its first two titles *before* these, so nothing moves above them.

## Backlog

- Stage 5: `nvs.template.format`, boolean, default `true`, into the manifest's frozen roster and
  read — the goal's § *Stage 5*, `rule:ide/contributions-are-frozen-and-only-ever-added`.
- Stage 6: rename the rule id, fill `guardedBy`, `python tools/rules.py --render` — the goal's
  § *Stage 6*.
- Emmet and HTML validation inside a region are still unreached — `docs/agent/carried-gaps.md`.
