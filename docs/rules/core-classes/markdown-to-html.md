`Core\Markdown` reads one flavour, CommonMark 0.31.2 with the GFM extensions, and writes HTML only as a
`Core\Html\Markup` built from the same element list `Core\Html::sanitize` rebuilds through. That makes it
a launderer onto the HTML sink beside `escape` and `sanitize`, and the second one that takes a
runtime-computed string.

**The members, and no other door.**

- `Core\Markdown::toHtml(string $markdown, ...callbacks): Core\Html\Markup` renders a whole document.
- `Core\Markdown::toHtmlEach(Core\Bytes::Source|Core\Bytes::Chunks $markdown, ...callbacks):
  iterable<Core\Html\Markup>` renders one block group at a time (`rule:core-classes/markdown-chunks`).
- `Core\Markdown::live(...callbacks): Core\Markdown\Live` renders text that arrives in pieces
  (`rule:core-classes/markdown-chunks`). It is an object because it has a lifetime
  (`rule:core-api/a-lifetime-is-an-object`).
- `Core\Markdown::fromHtml(string|Core\Html\Markup|Core\Xml\Node $html): string` writes Markdown
  (`rule:core-classes/markdown-from-html`).

No member takes an options bag. `toHtml` and `toHtmlEach` are the whole and the streamed shape of one
job, as `Core\Json::decode` and `decodeEach` are (`rule:core-classes/json-whole-and-stream`).

**One flavour.** CommonMark plus tables, task lists, strikethrough, extended autolinks and footnotes.
Front matter, math and admonitions are not parsed, so CommonMark's own reading of them is the output,
and a fenced block with a language reaches the `code` callback. Where GFM 0.29's spec text and
CommonMark 0.31.2 disagree, 0.31.2 wins: nine of GFM's emphasis examples predate 0.31.2's delimiter
rule, and the GFM spec test takes CommonMark's output for those nine. That rule counts a Unicode
symbol beside a delimiter as punctuation, and U+FFFD, which replaces NUL, is one. Its declaration,
`<!` and a letter, needs no whitespace after the name. GFM's tag filter is
not applied, because raw HTML goes through the rebuild below and the filter's tags never survive it.
Extended autolinks are read as cmark-gfm reads them, where comrak differs: the period of `www.` counts
as the domain's period, and a host may hold any character outside ASCII. They are always on, so the
five CommonMark examples that write a bare URL or email address as text (602, 606, 608, 611 and 612)
write it as a link, as GitHub does. A loose task item's box is written right after `<li>`, in front of
its first paragraph, as cmark-gfm writes it. Raw HTML in an image's description is its source text in
the `alt`, as cmark writes it. An HTML block of type 7 cannot interrupt a paragraph, and that holds for
a lazy continuation line too, as in CommonMark 0.31.2 and cmark. cmark-gfm and comrak start the block
there. That is a CommonMark construct and not a GFM one, so the CommonMark reading wins.

**Footnotes are written at the end**, after an `hr`, as an `ol` in the order they are first referenced.
A definition nothing references is not written. Each reference is a `sup` holding a link to its note,
and each note ends with a link back to its first reference.

**Raw HTML is rebuilt, never stripped and never passed through.** A document's raw HTML is parsed with
the rest of its output by the WHATWG algorithm and rebuilt through `rule:core-classes/html-sanitize`'s
list. A document with no raw HTML skips the rebuild, because the renderer writes only elements and
attributes on that list. `sanitizing_the_output_again_changes_nothing` is the property that makes the
skip safe.

**The list grows by what Markdown's output needs, each entry with a closed value.** CommonMark and GFM
output carries five things the list did not grant, and each is granted only with a value test:
`align` on `td` and `th` is `left`, `center` or `right`; `class` on `code` is `language-` followed by
letters, digits and `+-_#.`; `input` is written only with `type="checkbox"`, and carries `disabled`
and `checked` and nothing else; `id` on `h1` to `h6`, `a` and `li` starts with `md-`. The prefix holds
a hyphen, so no `id` Markdown writes can name a script's global variable, which is the DOM-clobbering
risk the list refused `id` for. A heading's `id` is `md-` and the `heading` callback's answer, and a
footnote's anchors are `md-fn-N` and `md-fnref-N`.

**URLs are read the way a browser reads them.** One function decodes entities and percent-encoding,
removes tab, line feed and carriage return, and checks the result against the sanitizer's scheme list
and its scheme-relative rule. A link whose URL fails is written as its text, and an image whose URL
fails is not written. A URL a callback returns goes through the same function.

**Attributes are never joined from strings.** The renderer builds the tree `Core\Html` serializes, so
an attribute value is escaped by the same code, and a heading's text cannot become an attribute.

**Callbacks** are named parameters, each optional:

- `link: fn(string $url): ?string` gives the new URL; `null` writes the link's text alone.
- `image: fn(string $url): ?string` gives the new URL; `null` removes the image.
- `heading: fn(int $level, string $text): ?string` gives the heading's `id`, after `md-`.
- `code: fn(string $language, string $code): ?Core\Html\Markup` gives the block's HTML; `null` keeps
  the default. A `Markup` is trusted by its type, so the result is used as it is. This is the one door
  for syntax highlighting, math and diagrams.

`link` sees every link, the extended autolinks too, with an email autolink's `mailto:`. A heading's
text is its character data, with no markup. `code` sees a fenced block that names a language, with the
language written as its class would be and the block's text. A callback that throws ends the render,
and the error reaches the caller.

**A `code` result is not rebuilt.** A rebuild would remove what the program wrote, such as a
highlighter's `style`, so the renderer writes a token in the block's place: a random 128-bit prefix
drawn for each render, which a document cannot write, and the fragment's index. The rebuild a
document's raw HTML takes runs over the token, and each fragment is written where its token is
afterwards. So `sanitizing_the_output_again_changes_nothing` holds for what the document wrote, and a
`code` result is outside it, as `Markup + Markup` is. **Declined:** rebuilding the result, which takes
away the one door this callback is for.

**The engine is `pulldown-cmark`.** It was measured against `comrak` over the 652 CommonMark examples,
the 670 enabled GFM examples and the 441 to-html entries of the known-bug catalogue
(`tests/data/markdown/known-bugs.toml`), in decision order:

| | pulldown-cmark 0.13 | comrak 0.59 |
|---|---|---|
| CommonMark examples failing, quotes in text aside | 0 | 0 (4 only with the autolink extension on) |
| GFM examples failing | 23: 11 extended autolinks, 9 emphasis, 2 tables, 1 tag filter | 14: 9 emphasis, 4 autolinks, 1 tag filter |
| Catalogue panics | 0 | 0 |
| Catalogued slow inputs that are not linear | 0 of 180 | 2 of 180: the unclosed-autolink inputs of CVE-2023-22483 |
| Reference resolved outside the text | the broken-link callback | the broken-link callback |
| A 6.5 MB document | 111 ms | 367 ms |

Every pulldown-cmark failure is written in our layer or is a small patch. NUL, a leading byte-order
mark, the table and code-block attributes, unused footnotes, extended autolinks and the output budget
are ours. The patches are tabs after an ATX closing sequence and a closing fence, a vertical tab in an
indented line, a definition-only list item, a DEL byte in a link destination, indentation inside a
code span on continuation lines, emphasis inside a footnote label, strikethrough's `~` and `~~`, which
open and close as `*` does in GFM and keep one lower bound each, a `~` closer, which matches the
nearest `~` opener or is text, a hard line break that only spaces make, not
tabs, an ATX heading whose `#` a vertical tab or form feed follows, which is text, a task item
with nothing after its marker, whose next line is a lazy continuation and which may end the
document, a table whose header row
has no pipe, which is one cell, a delimiter-row cell, which is GFM's: an optional colon, hyphens
and an optional colon, between spaces, and a paragraph with a delimiter row that made no table, which
no later line turns into a table, as in cmark-gfm, a header row indented four columns or more,
which still starts a table, the entities of an autolink, which resolve as cmark resolves them, and a tab that makes a block quote marker on a
continuing line indented four columns, which ends the quote. One patch adds what
`toHtmlEach` needs (`rule:core-classes/markdown-chunks`): a parser over one segment reads the
document's reference and footnote definitions and its expansion fuel from outside its text, and
reports where its top-level blocks start and end after the block pass alone. comrak's quadratic autolink
scan fails criterion (b), and it is three times slower. A patched engine is vendored under `vendor/`
with an entry in `[patch.crates-io]`, as `html5ever` is.

**Declined:**
- **An options bag or a flavour switch.** Each option doubles what must be tested and what a reader of
  a call must know. The callbacks cover what programs change.
- **An "unsafe" switch that passes raw HTML through.** It is the one switch every Markdown library's
  advisories trace back to.
- **Stripping raw HTML.** It loses content the rebuild keeps safely.
- **`class` and `id` with any value.** The page that receives the output selects on both.
- **Our own parser.** Both candidates pass the catalogue with patches, and a third parser is a third
  thing to fuzz.
- **AST editing.** The callbacks cover the four changes programs make, and a tree API is a later goal's
  question.
