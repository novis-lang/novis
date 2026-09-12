# Handoff

## State

**Goal 46 — format-on-save formats the markup too, from where the Novis code is — has just started; nothing of it has landed yet.** Goal `fmt`'s whole list is this goal's Stage 1 floor.

**The design is already decided and landed.** [ADR 0173](../../decisions/0173.md) holds all of it and
`rule:ide/a-template-region-gets-the-editors-services-and-formatter` states it at `designed`; no session
writes a record for this goal. The one thing not to re-decide: **the client computes no boundary** —
chunks come from the server's regions, and a failure anywhere degrades to less formatting, never to an
edited hole.

`nvs fmt` never touches markup and puts a `?>` that begins its line at its block's depth (goal `fmt`), so
the base column of every chunk is already in the text `nvs fmt --stdin` answers. What the server lacks
is a way to be asked about that text before the buffer holds it, which is this goal's keystone.

## Next group

**Stage 2: `nvs/regions` answers for a text it is given** — one file set: `crates/nvs-lsp/`.

- [ ] **The params** — `crates/nvs-lsp/src/server.rs:474`: `{textDocument, text?}`; with `text`,
      `regions::for_source` (`crates/nvs-lsp/src/regions.rs:96`) over it and not the buffer.
- [ ] **The guard tests** — `crates/nvs-lsp/tests/regions.rs`:
      `a_regions_request_carrying_text_answers_for_that_text_and_not_the_buffer` and
      `a_regions_request_without_text_answers_for_the_open_document`.
- [ ] **The module doc** — `crates/nvs-lsp/src/regions.rs:1-65`: the request's new param, and
      § *Anything at all about formatting* rewritten (stage 0's second sentence).

## Backlog

- Stages 3 and 4 — `editors/vscode/src/format.ts` (new), `template.ts`, `extension.ts` and
  `test/surfaces/`. One file set; stage 4 is the larger.
- Stage 5 — `editors/vscode/package.json` and the roster test. Cheap beside stage 4.
- Stage 6 — the rename across every citation, then the flip. Its own session: it touches every file that
  cites the id.
- When this goal's last check goes green the driver takes goal `gap-zero`.
