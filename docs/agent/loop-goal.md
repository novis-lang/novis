---
milestone: M10
---
# Loop goal 46 — format-on-save formats the markup too, from where the Novis code is

Format-on-save in a `.nvs` file runs `nvs fmt`, then the editor's own HTML formatter over each markup
chunk, and each chunk starts at the indentation of the `?>` line that opened it. A template reads with its
markup nested inside the Novis block it belongs to, and nothing jumps left and right between the two
modes. `nvs.template.format` turns the markup half off.

[ADR 0173](../decisions/0173.md) decided all of it and
`rule:ide/a-template-region-gets-services-but-no-second-formatter` states it. This goal is the
implementation, and its last stage renames that rule — whose id still names the half the record
removed — and flips it to `shipped`.

## Why here

After goal `fmt`, because the pass's first step is `nvs fmt --stdin` and
`rule:ide/every-feature-is-staged-behind-its-dependency` makes format-on-save wait for the formatter. The
base column is read off the `?>` layout that goal ships (`rule:tooling/fmt-novis-constructs`), so the
two goals cannot swap.

Before goal `gap-zero`, for that goal's standing reason.

What it needs already built: `nvs/regions` and the client's forwarding (goal `editor-surfaces`) —
`regions::for_source` (`crates/nvs-lsp/src/regions.rs:96`), the request's arm
(`crates/nvs-lsp/src/server.rs:474`), `virtual` (`editors/vscode/src/template.ts:103`) and
`install` (`editors/vscode/src/regions.ts:101`).

## Stage 0 — the catch-up

Sentences on disk that say no formatter reaches markup. Each is corrected in the stage that makes it
wrong — stage 3 for the first three, stage 6 for the id. Re-grep before editing: these are anchors, and
files move.

- `editors/vscode/src/regions.ts:10-15` — *"Nothing is registered as a formatting provider, and that
  absence is the rule's load-bearing half."*
- `crates/nvs-lsp/src/regions.rs:53-59` — § *Anything at all about formatting*.
- `editors/vscode/test/surfaces/template.test.ts:38-` — the assertion that no module under `src/`
  registers a formatting provider. It narrows to: `format.ts` registers exactly one, and nothing else
  does.
- The rule id. `ide/a-template-region-gets-services-but-no-second-formatter` becomes
  `ide/a-template-region-gets-the-editors-services-and-formatter`, with every citation
  `python tools/rules.py --citations` lists — code comments, `.lspt` cases, `docs/`, `website/`.
  `rules.py --check` refuses a dangling one, so the rename is one commit.

`the_server_declares_no_formatting_provider` stays true and is not touched: the server still declares
none, and the client's provider starts `nvs fmt`.

## Stage 1 — the floor

Goal `fmt`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: `nvs/regions` answers for a text it is given

The request's params become `{textDocument, text?}` at `crates/nvs-lsp/src/server.rs:474`. With `text`,
`regions::for_source` runs over a `SourceFile` built from it and the open buffer is not read; without
it, the answer is the buffer's, as today. The answer's shape does not change (`regions::wire`,
`crates/nvs-lsp/src/regions.rs:123`), so
`a_region_answer_carries_a_span_and_a_language_and_nothing_else` stands. ADR 0173 § 4 is why the
server and not the client.

## Stage 3 — the Novis half: format-on-save starts `nvs fmt`

A new `editors/vscode/src/format.ts`: a document formatting provider for `nvs` that runs
`nvs fmt --stdin` through `binary()` (`editors/vscode/src/binary.ts:15`) and answers one whole-document
edit, or no edit when `nvs fmt` refuses. It is installed beside `regions.install`
(`editors/vscode/src/extension.ts:101`). The extension still formats nothing itself: it starts the
formatter (`rule:ide/one-server-two-thin-clients`).

## Stage 4 — the markup half: chunks, the base, and the holes

The decisions go in `editors/vscode/src/template.ts`, which imports no `vscode`, so the headless tier
runs them (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`):

- **The chunks** — built from the formatted text and its regions: the markup between a `?>` that ends
  its line and the `<?nvs` that reopens code, the holes inside it included (ADR 0173 § 2).
- **The base** — the indentation of the line holding the chunk's opening `?>`, column zero before a
  file's first open tag, and the line holding the closing `<?nvs` at the base too (§ 3).
- **The stand-in** — what the formatter is shown in a hole's place, the same length, so every position
  maps back unchanged. Which stand-in is this stage's choice; the guard is that no hole byte is ever
  edited.
- **The merge** — the formatter's output re-based onto the chunk's base, whatever column the formatter
  started at, so the result does not depend on how the HTML service treats a range's first line; and a
  chunk refused whole when an edit reaches a hole.

`format.ts` is the `vscode` half. It asks `nvs/regions` with the formatted text, runs the editor's `html`
range formatter over a virtual document per chunk with `{ tabSize: 4, insertSpaces: true }`, and hands the
results to `template.ts`. The formatter is injected into the pure half, so every case runs headless
against a stub.

## Stage 5 — the setting

`nvs.template.format`, boolean, default `true`, added to the manifest's frozen roster
(`rule:ide/contributions-are-frozen-and-only-ever-added`) and read. The floor's "every contributed command
is registered, and every setting is read" check fails until it is. `false` leaves stage 3 alone.

## Stage 6 — the rulebook

Rename the rule id (stage 0's last item), flip it to `shipped` with `guardedBy` filled from this goal's
tests and goal `editor-surfaces`'s, and `python tools/rules.py --render`. Correct any of stage 0's
sentences stage 3 did not.

## Standing decisions

- **The design is ADR 0173's and is not re-opened**: the order — `nvs fmt`, then the regions, then the
  HTML formatter — on by default, the base from the `?>` line, and holes never edited. Do not add an
  option that changes the base or the unit.
- **The client computes no boundary.** Chunks are built from the server's regions and the text's line
  starts, never from a grammar, a regex over `<?` or a re-lex. If a chunk cannot be built from the
  regions alone, the fix is in the server's answer, recorded in `crates/nvs-lsp/src/regions.rs`'s module
  doc.
- **Refuse, never guess.** `nvs fmt` refusing leaves the buffer unchanged; the regions request failing
  leaves the markup as `nvs fmt` wrote it; an edit reaching a hole leaves that chunk as written. Every
  failure degrades to less formatting, never to an edited hole or a changed Novis byte.
- **The unit is four spaces**, passed to the HTML formatter whatever the editor's `tabSize` is, because a
  chunk's nesting and the code around it must agree on what a level is (ADR 0173 § 3).
- **Markup-literal bodies are out of this goal.** A chunk is markup between `?>` and `<?nvs`; a markup
  literal's body is not one, and it stays exactly as `nvs fmt` leaves it. Making it a region for the
  services is `crates/nvs-lsp/src/regions.rs` § *What is not a region yet*, not this goal.
- **PhpStorm is not touched.** ADR 0173 § 6: markup bytes are each editor's own.
- **The real formatter is the host run's.** VS Code's HTML formatter does not exist headless. Every loop
  check injects a stub, and a real template round-trips in the milestone's host run, not here.
- **No new ADR.** ADR 0173 is the design.
- **What it spends**: per format request, one `nvs fmt` process, one regions round trip carrying the
  file's text once more, and one HTML format per chunk. Nothing is held between requests, and the server
  does nothing beyond the one lex it already does.
