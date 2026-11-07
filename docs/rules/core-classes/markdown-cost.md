`Core\Markdown`'s work is linear in its input, its nesting is bounded by the HTML parser's own bound,
and the output it adds beyond its input is bounded by the input's length plus 64 KiB. Each of these is
a property of the parse, so no input is refused and no parse fails: Markdown has no invalid input.

**Work is linear.** Every catalogued slow input of every library in the known-bug catalogue is a test
that counts work steps, as `parsed_counting_steps` does for HTML, and fails when the steps grow faster
than the input. There is no timing in the test and no timeout in the member, so a slow input is a bug
found in the tree, not a request cut off in production. A step is one pass of a loop in the engine, one
byte one of its scanning primitives reads, and one byte of HTML written; the vendored engine counts its
own in a build with debug assertions, and a release build compiles the count away. Each input renders
at `n` and `4n` repeats, and the steps at `4n` must be more than at `n` and at most five times as many.
A count that misses the loop an input grows is the one way this test can pass a slow input, so a
loop added to the engine by a patch counts its steps too.

**Nesting deeper than 512 flattens exactly as `rule:core-classes/html-parsing` flattens it.** It is the
same bound and not a new one: the output passes through, or must equal, the WHATWG parse, and a tree
the renderer wrote deeper than the parser keeps would change when it is sanitized again.

**Added output** is output that does not come from the input's bytes one for one: a reference's URL
and title written at each use, and the empty cells that pad a short table row. One document adds at
most its own input length plus 64 KiB. Past that, a reference renders as its source text and a row is
not padded. No real document comes near the bound, and without it a 1 MB input writes gigabytes,
which seven of the catalogued libraries shipped.

**NUL becomes U+FFFD**, as the spec says, before the engine reads the text. Invalid UTF-8 follows
`rule:types/string-is-utf8`. A byte-order mark at the start of the document is not text.

**A panic is a bug.** The fuzz targets run the parse and the renderer, and a differential fuzz runs
the other engine the choice was measured against. Each difference is triaged into our bug, fixed and
catalogued, or a spec reading, one line in `rule:core-classes/markdown-to-html`. Where comrak 0.59
departs from the spec, the differential does not compare: footnotes (written as this rule writes them),
extended autolinks, among them an email whose `@` is an entity, which cmark-gfm reads in the
decoded text and comrak 0.59 in the source, a delimiter beside a Unicode symbol, `~` beside `*` or `_`, a `~` run beside a `~~` run, a declaration, a definition's title with text after it on its line, which cmark now drops and comrak 0.59 keeps,
a `\` before a link title's closing delimiter, which cmark reads as an escape since its
issue 607 and comrak 0.59 may read as a character, a processing instruction that ends `??>`, an HTML comment
whose text ends with `-`, which 0.29 did not allow, a CDATA section with `]]]`, which cmark's pattern reads
past its end, `<![CDATA[` in lower case, which comrak reads as HTML and cmark as text, a `---` under a paragraph of reference definitions alone, which cmark now reads as a
thematic break, a task marker with an inline
right after its `]`, a task marker that starts a line with no list marker before it, which cmark-gfm's
scan of the whole line does not read as a box and comrak 0.59 does, a destination in `<…>` that starts or ends with whitespace, which cmark trims and the spec
keeps, a link label near 1,000 bytes,
which the spec limits to 999 characters and cmark to 1,000 bytes, a second link label of whitespace alone, as in `[a][ ]`, which the spec reads as
text after a shortcut reference and cmark as a collapsed reference, a lazy line that would start an HTML block of type 7 or starts with spaces inside a code span, nesting past comrak's depth of 100, a `\&` before an entity in a link
destination, which cmark decodes and the spec and commonmark.js read as text, a run
of two or more backticks, a `\\|`, where cmark-gfm removes the `\` in front of a table cell's `|`
and comrak 0.59 keeps it after a pair of backslashes, a `\|` in a code span or raw HTML of a
paragraph a table interrupts, where cmark-gfm removes the `\` and CommonMark keeps it, a `\r\n` in a code span, which comrak writes as two spaces, a blank line after a thematic break in a list item, which makes
the list loose in current cmark and not in cmark-gfm, a `~~~` fence beside a lone `\r`,
after which comrak drops a line that holds only the fence's indentation, and a vertical tab or form feed, which 0.29 counted as whitespace. comrak writes a `%` with no two hex
digits after it in a URL as `%25`. cmark and this renderer write it as it is, and the target maps that
difference away. comrak 0.59 lets references add at most 100,000 bytes and at most the document's
length, where cmark allows at least 100,000, so the target gives comrak a longer document when a
reference is text. A raw `<a>` tag is not compared either, since the target's mapping of a nested
link cannot follow the parser's close of it, and for the same reason neither is an angle autolink
beside raw HTML in a document with a link. Nor is a document where comrak writes a link inside a link,
as it does for `[[a](u)[b]](v)`, which cmark and the spec write as `[`, the link `a` and text, or a
link around an image whose text holds a link, as in `[![[]()[]]()]()`. The target's comments give each case. The other two targets cover these documents.

**The differential is a tool run by hand, not a check.** `bun nv fuzz markdown_comrak` runs it, and
CI's fuzz lane runs only `markdown` and `markdown_from_html`. Its triage stopped once its finds were
cmark-gfm conformance edge cases with no security value; the security finds, two sanitizer
fixed-point fixes, landed. So on unusual input the output may still differ from cmark-gfm's, as
`[a]:\n-|-\n-|-` does, where cmark-gfm reads a definition and ours a paragraph. Such a difference is
fixed when a user meets it, with a catalogue entry. In CI those open differences would turn the deep
run red at random, which says nothing about safety.

**Memory:** `toHtml` holds the document, its output tree and its reference and footnote definitions,
all attributed to the request and freed with it. `toHtmlEach` and `Live` hold less, as
`rule:core-classes/markdown-chunks` says. The added-output budget is one counter per parse.

**Declined:**
- **A timeout or a worker** in place of linear work. Either one turns a quadratic path into a request
  that fails late, and still spends the time.
- **A depth setting.** It could only buy back the quadratic cost the bound removes.
- **Refusing a document that hits a bound.** A document is still rendered, and the bound decides only
  how much of the extra output exists.
