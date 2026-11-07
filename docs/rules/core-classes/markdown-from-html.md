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

**What Markdown cannot write** — a table outside GFM's shape, `dl`, `figure`, and an `ol` whose start
has more than nine digits — is written as its sanitized HTML on one line, with each line feed as
`&#10;`, so CommonMark reads it as one HTML block that no blank line ends early. An inline element
with no Markdown syntax, such as `sup`, is its tags around Markdown content.

**Depth.** The walk follows 128 levels of elements, and past that writes an element's text alone. It
bounds both the native stack and the copy each level makes of its content, which keeps work linear.

**The writing rules.**

- Whitespace collapses by the CSS rules for inline and block elements. CSS white space is a space, a
  tab and a line break; a form feed is a character a browser shows, so it stays in the text.
- A `<br>` is written where a browser shows a line for it. A browser shows no line for one break
  alone at the end of a block with content before it, so that one is dropped. Two or more there are
  all written, so the result is the same when it is read and written again. A break between two pieces of text is a backslash at the end of the
  line. Markdown has no syntax for a break with no text before or after it in its block, so that one
  is `<br />`, inline HTML.
- A GFM table is written when every cell holds only inline content and no cell uses `colspan` or
  `rowspan`. Any other table is written as sanitized HTML.
- A `div`, and any other container with no Markdown syntax, is unwrapped: its children are written as
  blocks of the container around it. An element with no content, such as an empty paragraph or list,
  is written as nothing, since a browser shows nothing for it.
- A code block's fence is one backtick longer than the longest run of backticks inside it.
- A URL goes through the same check `rule:core-classes/markdown-to-html` applies, so a link the
  sanitizer would drop is written as its text.
- A URL is written percent-encoded as the renderer writes an `href`, so `/Straße` is `/Stra%C3%9Fe`.
  Written as it was, it would render to the encoded `href`, and `fromHtml` run twice would differ. It
  costs readability for a URL with a space or a non-ASCII character. Declined: decoding an `href` to
  readable text, which changes the URL where it holds an encoded `/` or `%`.

`fromHtml` run twice gives the same Markdown, and Markdown rendered to HTML and written back keeps its
meaning. Both are tests. The second holds for Markdown with no raw HTML: raw HTML comes back as Markdown
where Markdown has the construct, so a `div` it holds does not come back.

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
