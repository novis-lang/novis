# Handoff

## State

**Goal `editor-surfaces` — milestone M10. Every stage is closed: 0, 2, 3, 5 and 6.** Stage 6's client
half landed this session, which was the goal's last open check. `npm run test:headless` in
`editors/vscode` is 213 passing, `python tools/reference.py --check` is current, and the two stage-6
Rust checks were already green.

**The forwarding is two files, split the way `redactions.ts`/`concealment.ts` is.**
`editors/vscode/src/template.ts` holds the decisions and imports no `vscode`, so the headless tier
runs them: which spelling maps to which service, whether a position is inside a region, and the
virtual document a service is shown — the file with its regions where they are and whitespace
everywhere else, so a position means the same thing in both and no range coming back needs mapping.
`editors/vscode/src/regions.ts` is the half that asks `nvs/regions` and registers the four providers.
The regions are asked for per request rather than held, which is the one place it departs from
`redactions.ts` and the module doc says why.

**Two of the services the rule names do not arrive by forwarding.** Emmet expands from the language
of the document the cursor is in and validation is published per document by the service that owns
one, so neither is a provider that can be run over a virtual document. Nothing is half-registered for
them; the decision is filed in `docs/agent/carried-gaps.md` § *Unowned*.

## Next group

**The goal's checks are all green, so the driver switches to goal `resource-ceilings` and replaces
this file with that goal's seed handoff.** If a check disagrees and this goal stays live, the one
item below is what is left of stage 6 — one file set, `editors/vscode/src/`.

- [ ] **Emmet and HTML validation reach a template region, or the tree says why not**, per
      `rule:ide/a-template-region-gets-services-but-no-second-formatter`, which names both. The two
      candidate shapes and the cost of each are in `editors/vscode/src/regions.ts:23`; the decision
      is whether `emmet.includeLanguages` mapping `nvs` to `html` — which also turns abbreviation
      expansion on in the Novis half of the file — is the trade, or whether the client grows a
      second, real document the service can own beside `editors/vscode/src/template.ts:103`.
- [x] **`nvs.template.services` joins the frozen roster** — `editors/vscode/package.json:133`, both
      contributions rows, and the reference chapter's settings table.
- [x] **The client forwards inside a region and nowhere else** — `editors/vscode/src/regions.ts:100`
      registers completion, hover, linked editing and colours, and no formatter on either side.

## Backlog

- Emmet and HTML validation do not reach a region — `docs/agent/carried-gaps.md` § *Unowned*.
- A markup literal's body is not a region: the lexer has no token for one — `crates/nvs-lsp/src/regions.rs` § *What is not a region yet*, goal `markup-literal`.
- `nvs.lsp.debounce` is contributed and read by nothing — `docs/reference/tools/40-editor.md` § *What it does not do*.
- Coverage in the Test Explorer waits on the Clover/lcov exporters — goal's § *Standing decisions*.
