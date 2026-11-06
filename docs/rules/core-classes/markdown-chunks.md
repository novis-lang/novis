`Core\Markdown::toHtmlEach` splits a document into top-level block groups and renders each group alone,
and the HTML of the groups joined is the HTML of the whole document. `Core\Markdown\Live` renders text
that arrives in pieces, and a block it has finished never changes.

**A block group ends only where nothing is open.** A boundary is never placed inside an open fenced
code block, an HTML block of types 1 to 5, an unclosed raw HTML element, or a list that continues. A
document that keeps one open, such as one huge list, becomes one larger group: correctness first.
`rendering_in_chunks_gives_the_same_html_as_the_whole_document` is the acceptance.

**References may be defined after their first use.** A `Core\IO\File` is read twice: once to collect
the reference definitions, once to render. Every other source is read once and held whole under the
request's memory cap, as a random read of an upload part already is. Footnotes are written at the end,
so their definitions are held until then. The source is read through the one byte source every such
member uses (`rule:core-api/a-member-takes-the-source-the-program-holds`).

**`Live`** takes `push(string $piece)`, which returns an object with `done: Core\Html\Markup` (the
blocks that are finished, to append) and `open: Core\Html\Markup` (the unfinished last block, to
replace). `finish(): Core\Html\Markup` closes the document. A block is rendered as soon as it is
closed, so a reference defined after a finished block does not change that block. The `done` parts
and `finish()` joined are the whole document's HTML for a document with no forward reference, and
`live_never_changes_a_part_it_already_finished` holds for every document.

**Memory:** a file costs one block group plus its reference and footnote definitions. Any other source
costs the document. `Live` costs its open block plus the definitions. All of it is attributed to the
request.

**Declined:**
- **Re-rendering a finished `Live` block when a later reference resolves it.** A caller that appended
  the block cannot take it back, and a part that changes is a part the caller must diff.
- **Boundaries inside an open construct, repaired afterwards.** A repair is a second parse with its own
  bugs, and the property would then hold only as well as the repair.
- **Reading a non-file source twice by buffering it to disk.** It trades a bounded memory cost for
  disk work on the request path, and the request's cap already bounds the memory.
