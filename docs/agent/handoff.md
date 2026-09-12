# Handoff

## State

**Goal `template-format` (M10) — stage 2, the keystone, has landed.** `nvs/regions` now takes
`{textDocument, text?}`: with a `text` it answers for that text, without one for the open buffer, and
the answer's shape is unchanged. `crates/nvs-lsp/src/regions.rs` holds the params, the choice and the
reasoning (`Params`, `for_request`, and the module doc's § *the text comes with the question*);
`crates/nvs-lsp/src/server.rs`'s `template_regions` is the wire wrapper.

**The wire shape changed, so the client changed with it**: `editors/vscode/src/regions.ts` sends
`{textDocument: {uri}}` and no `text`, since it asks about the open buffer. `nvs/redactions` still
takes a bare `{uri}` — only this request needs a text.

Stage 1 (goal `fmt`'s whole list) is the floor and is closed. Nothing of stages 3–6 exists: the
extension has no formatting provider, `template.ts` has no chunking, and `nvs.template.format` is not
in the manifest. Nothing is blocked; the design is ADR 0173's and is not re-opened.

## Next group

**Stage 3: the Novis half — format-on-save starts `nvs fmt`** — one file set: `editors/vscode/src/`.

- [ ] **The provider** — `editors/vscode/src/format.ts` (new, beside `editors/vscode/src/regions.ts:68`):
      a `DocumentFormattingEditProvider` for the `nvs` selector that runs `nvs fmt --stdin` through
      `binary()` (`editors/vscode/src/binary.ts:15`) and answers one whole-document edit, or **no** edit
      when `nvs fmt` refuses (`rule:ide/one-server-two-thin-clients`; the goal's § *Stage 3*).
- [ ] **The install** — `editors/vscode/src/extension.ts:101`: `format.install(context)` beside
      `regions.install(context)`, and `activate` still formats nothing itself.
- [ ] **The case** — `editors/vscode/test/surfaces/template.test.ts:51`: the titles the stage-4 check's
      `want` list opens with — "the template format", "leaves the buffer unchanged when nvs fmt
      refuses" — asserted headless with the process stubbed
      (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`).

## Backlog

- Stage 4's chunks, base, stand-in and merge, all in `template.ts` — the goal's § *Stage 4*.
- Stage 5's `nvs.template.format` setting, into the frozen roster — the goal's § *Stage 5*.
- Stage 6's rule rename, `shipped` flip and `python tools/rules.py --render` — the goal's § *Stage 6*.
- A markup literal's body is still not a region — `crates/nvs-lsp/src/regions.rs` § *What is not a
  region yet*.
- The real HTML formatter only runs in the milestone's host run; the loop checks a stub — the goal's
  § *Standing decisions*.
