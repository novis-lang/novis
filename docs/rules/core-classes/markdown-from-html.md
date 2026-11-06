`Core\Markdown::fromHtml` writes Markdown from HTML, and HTML text never becomes Markdown syntax: every
text node is escaped, and a construct Markdown cannot express is written as sanitized HTML.

**Input** is a string or a `Core\Html\Markup`, parsed by `Core\Html::parse`, or a `Core\Xml\Node`
already parsed. Elements are kept or dropped by `rule:core-classes/html-sanitize`'s list, so a
`<script>`'s text never appears. A tainted input gives a tainted result.

**The writing rules.**

- Whitespace collapses by the CSS rules for inline and block elements.
- A GFM table is written when every cell holds only inline content and no cell uses `colspan` or
  `rowspan`. Any other table is written as sanitized HTML.
- A code block's fence is one backtick longer than the longest run of backticks inside it.
- A URL goes through the same check `rule:core-classes/markdown-to-html` applies, so a link the
  sanitizer would drop is written as its text.

`fromHtml` run twice gives the same Markdown, and Markdown rendered to HTML and written back keeps its
meaning. Both are tests.

**Large documents.** A document is read as a stream when the tree builder can no longer change an
element once it is closed; the streamed result equals the result over the whole tree. Where the
builder can still change a closed element, `fromHtml` holds the tree, and this rule says why.

**Memory:** the document's tree, as `Core\Html::parse`'s is, and the Markdown it writes, attributed to
the request.

**Declined:**
- **Choosing a page's main content** (dropping navigation, footers and ads). The program passes the
  element it wants, and a guess at "main" is a heuristic every page defeats.
- **Writing raw HTML that the sanitizer would not keep.** The Markdown is read again later, by a
  renderer that may not rebuild it.
- **Options for the Markdown style.** One style is one thing to test, and `toHtml` reads every style.
