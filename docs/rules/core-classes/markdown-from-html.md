`Core\Markdown::fromHtml` writes Markdown from HTML, and HTML text never becomes Markdown syntax: every
text node is escaped, and a construct Markdown cannot express is written as sanitized HTML.

**Input** is a string or a `Core\Html\Markup`, parsed by `Core\Html::parse`, or a `Core\Xml\Node`
already parsed. Elements are kept or dropped by `rule:core-classes/html-sanitize`'s list, so a
`<script>`'s text never appears: the converter walks the sanitizer's rebuilt tree.

**The result is always `tainted string`.** A node's text is tainted, and a member cannot make its
result's mark follow one arm of a union, so a plain result from a node would launder that text for
every sink. A `Markup` input pays for it with a tainted result, which `toHtml` and `echo` take as they
are. Taking a node does not open a path back into execution, because the member returns text alone;
`a_parsed_tree_has_no_path_back_into_execution` allows exactly that shape.

**How text stays text.** `` \ ` * _ [ ] < > & | ~ `` always get a backslash, and `# - + = :` and the
`.` or `)` after a number get one at the start of a line, where `:-` would start a table's
delimiter row. A `!` right before a link's `[` gets one too, or the link would be an image. A backslash cannot stop a GFM extended
autolink, since the renderer finds those after escapes are read, so a text that would autolink is cut
with an empty comment `<!-- -->` before the `.` of `www.`, the `:` of `://` and the `@` of an address.
The sanitizer drops the comment. Emphasis whose content starts or ends with punctuation is written as
its tags, because `*` there is not read as emphasis.

**What Markdown cannot write** — a table outside GFM's shape, `dl`, `figure`, an `ol` whose start
has more than nine digits, and a `pre` whose text holds a carriage return, which Markdown reads as a
line ending — is written as its sanitized HTML on one line, with each line feed as `&#10;`, so
CommonMark reads it as one HTML block that no blank line ends early. That HTML is the sanitizer's
answer for the element, which the renderer writes unchanged: the input is parsed in quirks mode, where
`<table>` does not close a `<p>`, and the sanitizer is not, so a `p` around a `table` written as it
was would come back as two paragraphs and a table. Each such block costs a second parse. An inline element
with no Markdown syntax, such as `sup`, is its tags around Markdown content. An `<a>` nested in an
`<a>`, which the parser closes into links with no text, is written as `[](/x)`, where any other empty
element writes nothing. A link that starts a paragraph and whose text has a `]` with a `:` after it in
a code span, where no backslash can go, is written as its tags, since `[text]:` there starts a link
reference definition.

**Depth.** The walk follows 128 levels of elements, and past that writes an element's text alone. It
bounds both the native stack and the copy each level makes of its content, which keeps work linear.

**The writing rules.**

- Whitespace collapses by the CSS rules for inline and block elements. CSS white space is a space, a
  tab and a line break; a form feed is a character a browser shows, so it stays in the text.
- A `<br>` is written where a browser shows a line for it. A browser shows no line for one break
  alone at the end of a block with content before it, so that one is dropped. Two or more there are
  all written, so the result is the same when it is read and written again. A break between two pieces of text is a backslash at the end of the
  line. Markdown has no syntax for a break with no text before or after it in its block, so that one
  is `<br />`, inline HTML. A paragraph that is one such `<br />` alone is a line CommonMark reads as
  an HTML block, which runs on to the next blank line, so it is placed as an HTML block. A link with
  no text shows nothing, so the breaks in it are written in front of it.
- A list item's blocks share lines where the next block cannot be read as part of the one before,
  since a blank line between two of them makes the list loose. After a paragraph, a block that
  interrupts a paragraph goes on the next line, and so does a table: GFM reads the line before a
  delimiter row as the header row, and the table writes its own. After a quote or a list that ends in a paragraph, so
  does any list, which after a list has another marker: a marker there ends the paragraph even where
  it could not interrupt one. After a table, a heading, a fence, a rule, a quote and any list go on
  the next line, since each ends the table's rows, an empty item and a number other than 1 too. After an empty quote, or a list whose last block no line continues, any
  block that does not continue it goes on the next line. A paragraph, table or HTML block after a
  quote that ends in a paragraph or a table gets a last `>` line in the quote, which ends that block
  and keeps the item tight. When the quote ends in a list, that line makes the list around the
  quote loose, so a blank line goes there. A list whose item starts with a rule or a list takes the first of `-`, `*` and
  `+` that differs from the list before and does not make that line a thematic break, as `* ***` is.
- The footnote section the renderer writes, a last `<hr>` and an `<ol>` whose items have the ids
  `md-fn-1`, `md-fn-2` and on, each ending in its `↩` link, is written as footnotes: each reference
  `<sup><a href="#md-fn-n">n</a></sup>` as `[^n]`, and each item as `[^n]:` without its back link,
  which the renderer writes again. Only a footnote gives an element an id, so a list would lose them.
  The renderer numbers notes by their first reference in the source, also one inside a definition,
  and gives that reference alone the id `md-fnref-n`, so the body and the definitions are written in
  the order that gives each note its number and puts each id where it was. A definition may come
  before the body, as `[^1]: [^1]` does when only the note's own reference has the id. A note whose
  id no shown reference has was first named in a definition the renderer does not write, so `[^0]:`,
  a definition no reference names, names it at its place in that order. A section no order fits,
  such as references with ids out of the notes' order in one paragraph, is written as a list and
  links. A `(` or `:` right after a reference gets a backslash and a link an empty comment, or the
  reference would read as a link or a definition.
- A GFM table is written when every cell holds only inline content and no cell uses `colspan` or
  `rowspan`. Any other table is written as sanitized HTML.
- A `div`, and any other container with no Markdown syntax, is unwrapped: its children are written as
  blocks of the container around it. An element with no content, such as an empty paragraph or list,
  is written as nothing, since a browser shows nothing for it. A list is written loose when a `p`
  straight inside one of its items writes a block. A `p` that writes nothing does not count, since
  no paragraph shows for it loose or tight.
- A task list item's checkbox is written `[ ]` or `[x]`, with a space before a break after it: the
  renderer reads the marker only when white space follows it.
- A code block's fence is one backtick longer than the longest run of backticks inside it, and so is
  a code span's. A code span right after another gets an empty comment between them, since
  `` `a``b` `` is one span.
- A URL goes through the same check `rule:core-classes/markdown-to-html` applies, so a link the
  sanitizer would drop is written as its text.
- A URL loses the control characters and spaces at each end, and every tab and line break, as a
  browser's URL parser removes them. Encoded instead, `\x13http:` would be `%13http:`, which has no
  scheme, and the renderer would drop the link.
- A URL is written percent-encoded as the renderer writes an `href`, so `/Straße` is `/Stra%C3%9Fe`.
  Written as it was, it would render to the encoded `href`, and `fromHtml` run twice would differ. It
  costs readability for a URL with a space or a non-ASCII character. Declined: decoding an `href` to
  readable text, which changes the URL where it holds an encoded `/` or `%`.

`fromHtml` run twice gives the same Markdown, and Markdown rendered to HTML and written back keeps its
meaning. Both are tests. The second holds for Markdown with no raw HTML: raw HTML comes back as Markdown
where Markdown has the construct, so a `div` it holds does not come back. The `markdown_from_html` fuzz
target checks the first, and that nothing panics. The second is a fixed table of documents,
`markdown_to_html_to_markdown_keeps_the_meaning`. It is fidelity, not safety, and as a fuzz property it
found a new difference on nearly every run (`rule:testing/a-fuzz-search-gates-nothing`).

**Large documents: the parse streams, and the tree is held whole.** The WHATWG tree builder still
changes a top-level node after a later one has opened. Text that arrives inside a table before any
cell is fostered in front of the table, and joins the text node there: `a<table>b` is the text `ab`
and a table. A writer that wrote each finished top-level subtree would have written `a` alone. The
Markdown writer carries state across blocks too, since a list after a list changes its marker. And
streaming would buy only memory: every input is already whole in memory, a string, a `Markup` or a
node, and the result is one string, so the tree is the only thing it saves. Memory is the last
priority and the walk's simplicity is above it, so `fromHtml` holds the tree. What does stream is the
read: the parser takes a document in pieces, and the Markdown is the whole tree's for every cut,
which `from_html_streamed_gives_the_same_markdown_as_the_whole_tree` pins.

**Memory:** the document's tree, as `Core\Html::parse`'s is, and the Markdown it writes, attributed to
the request.

**Declined:**
- **Choosing a page's main content** (dropping navigation, footers and ads). The program passes the
  element it wants, and a guess at "main" is a heuristic every page defeats.
- **Writing raw HTML that the sanitizer would not keep.** The Markdown is read again later, by a
  renderer that may not rebuild it.
- **Options for the Markdown style.** One style is one thing to test, and `toHtml` reads every style.
- **Writing each top-level subtree as the parser closes it.** The builder can still change a closed
  node (*Large documents*), so its output would differ from the whole tree's, to save memory alone.
