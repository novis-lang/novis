`Core\Markdown::toHtmlEach` splits a document into top-level block groups and renders each group alone,
and the HTML of the groups joined is the HTML of the whole document. `Core\Markdown\Live` renders text
that arrives in pieces, and a block it has finished never changes.

**A block group ends only where nothing is open.** A boundary is never placed inside an open fenced
code block, an HTML block of types 1 to 5, an unclosed raw HTML element, or a list that continues. Nor
is one placed before a table whose header row is indented four columns or more: that row is read as a
header only after a paragraph, and alone it would be indented code. A document that keeps one open, such as one huge list, becomes one larger group: correctness first.
`rendering_in_chunks_gives_the_same_html_as_the_whole_document` is the acceptance.

**Raw HTML is read with a tokenizer that errs toward one group.** It keeps the elements raw HTML
opened, and an end tag closes only the innermost one it names. A comment, a raw-text element or a tag
left unfinished when Markdown output follows ends every later boundary. A group after the first is
rebuilt behind the first group's leading doctype and a `<body>` tag, with its leading whitespace kept,
so the parser reads it in the mode and place the whole document's parse did.

**References may be defined after their first use.** A `Core\IO\File` is read twice: once to collect
the reference definitions, once to render. Every other source is read once and held whole under the
request's memory cap, as a random read of an upload part already is. Footnotes are written at the end,
so their definitions are held until then. The source is read through the one byte source every such
member uses (`rule:core-api/a-member-takes-the-source-the-program-holds`).

**The engine reads the document in segments, and both passes cut the same ones.** A segment is a run
of whole top-level blocks: the text read so far is cut after its second-to-last block, at the next
line start, so the last block and a reference definition after the block before it can still grow.
Only complete lines are read, and the text is read again only once it has doubled, so one block as
long as the document costs linear work. The first pass reads each segment's definitions without its
inline content. The second renders each segment with the whole document's definitions, which win over
its own, and with one expansion fuel for the whole document. `rendering_in_chunks_gives_the_same_html_as_the_whole_document`
also reads every input 1 byte and 7 bytes at a time, which cuts a segment wherever one can be cut.

**`Live`** is `Core\Markdown::live(...callbacks): Core\Markdown\Live`. Its `push(string $piece)`
returns a `Core\Markdown\Update`, whose `done(): Core\Html\Markup` is the groups that piece finished
(to append) and whose `open(): Core\Html\Markup` is the rest rendered as if the text ended there (to
replace). They are members, not properties, because a `Core` instance has no property a program can
read. `finish(): Core\Html\Markup` closes the document, and a `push` or `finish` after it throws a
`RuntimeError`. A block is rendered as soon as the next block's first line is complete, so a reference
defined after a finished block does not change that block: Live has no first pass, and learns a
segment's definitions only after rendering it. The `done` parts and `finish()` joined are the whole
document's HTML for a document with no forward reference, and
`live_never_changes_a_part_it_already_finished` holds for every document. A callback runs again for a
block each time `open` renders it.

Live's segments cut at **every piece that completes a line**, not once the text doubles, so a block is
finished as soon as it can be. That parse costs the text not yet finished, which is the same order as
rendering `open` from it, which every `push` does anyway. One block pushed in many pieces therefore
costs work in the square of its length. That is the price of an `open` part after every piece, and
the program, which chose the pieces, pays it. A segment adds its own length to the added-output budget
and the reference fuel when it is rendered, since the document's length is not known up front.

**Memory:** a file costs one segment, its groups' HTML, and its reference and footnote definitions. A
segment is at most four reads of 64 KiB past its last block, because the text doubles before it is
read again and a string's capacity can be twice its length. Any other source costs the document as
well. `Live` costs its open block plus the definitions. All of it is attributed to the request.
`to_html_each_holds_one_block_group_of_a_file_not_the_document` holds the bound.

**A parse is asked of the memory limit before it runs.** Each parse of uncut text, to find a cut or
to read a segment's definitions, holds a tree of a few items per line. One segment can be large: a
run of reference definitions is one block, so many of them are one segment. So the request must
afford the text plus 512 bytes per line first, the most a line of short definitions was measured to
hold, or the call stops at the memory limit. A `Live` frees its state when the object is freed, so
one never finished costs nothing after its last use.

**Declined:**
- **Re-rendering a finished `Live` block when a later reference resolves it.** A caller that appended
  the block cannot take it back, and a part that changes is a part the caller must diff.
- **Boundaries inside an open construct, repaired afterwards.** A repair is a second parse with its own
  bugs, and the property would then hold only as well as the repair.
- **Reading a non-file source twice by buffering it to disk.** It trades a bounded memory cost for
  disk work on the request path, and the request's cap already bounds the memory.
