---
summary: the HTML of a Markdown document, one part at a time. `Core\Markdown::toHtmlEach` returns it. You read it with a `foreach` loop
keywords: markdown to html in parts, large markdown file, stream markdown, markdown chunks, low memory
---

`Core\Markdown\Groups` contains the HTML of a Markdown document, one part at a time.
`Core\Markdown::toHtmlEach` returns it. The class has no methods. You read it with a `foreach` loop.

Each item is a `Core\Html\Markup` with one or more whole blocks, such as a heading, a paragraph or a
list. A block is never cut in two. The parts joined together are the same HTML that
`Core\Markdown::toHtml` returns for the whole document.

```nvs
<?nvs
bytes $markdown = "# Notes\n\nFirst line.\n\n- one\n- two\n" as bytes;
foreach (Core\Markdown::toHtmlEach($markdown) as Core\Html\Markup $part) {
    echo "part:\n", $part;
}
```
```output
part:
<h1>Notes</h1>
part:
<p>First line.</p>
part:
<ul>
<li>one</li>
<li>two</li>
</ul>
```
