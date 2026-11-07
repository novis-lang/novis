---
summary: Markdown to HTML that is safe to put into a page — CommonMark with tables, task lists, strikethrough, autolinks and footnotes
keywords: markdown, commonmark, gfm, github flavored markdown, readme, render, to html, comments, user content, xss
---

`Core\Markdown::toHtml` converts a Markdown document to HTML. It reads CommonMark with the GitHub
extensions: tables, task lists, strikethrough, autolinks and footnotes. The result is a
`Core\Html\Markup`, so you can write it into a page without escaping it again.

The result is safe even when a user wrote the document. HTML inside the document is cleaned the same
way `Core\Html::sanitize` cleans it. A link with an unsafe URL, such as `javascript:`, is written as
plain text. An image with an unsafe URL is removed. Any text is a valid document, so the document
itself never causes an error.

```nvs
<?nvs
string $text = "# Notes\n\nSome *text*, a [link](https://example.com) and www.example.com.\n\n[bad](javascript:alert(1)) <img src=x onerror=alert(1)>\n";
echo Core\Markdown::toHtml($text);
```
```output
<h1>Notes</h1>
<p>Some <em>text</em>, a <a href="https://example.com">link</a> and <a href="http://www.example.com">www.example.com</a>.</p>
<p>bad <img src="x"></p>
```

## Changing links, images, headings and code blocks

`toHtml` takes four optional functions, which you pass by name:

- `link` gets each link's URL and returns the URL to write. If it returns `null`, the link is removed
  and its text stays.
- `image` gets each image's URL and returns the URL to write. If it returns `null`, the image is
  removed.
- `heading` gets each heading's level and text, and returns its `id`. The `id` is written after `md-`,
  so `intro` gives `md-intro`.
- `code` gets the language and the text of each fenced code block that names a language. The
  `Core\Html\Markup` it returns replaces the block. If it returns `null`, the block is written as usual.

A URL a function returns is checked like a URL in the document, so `javascript:` is still not written.
The `Markup` that `code` returns is written exactly as it is. Use `code` for syntax highlighting, math
or diagrams. If one of the functions throws an error, `toHtml` stops and your code gets that error.

```nvs
<?nvs
string $text = "## Install\n\nSee [the guide](/guide).\n\n```shell\nnvs run app.nvs\n```\n";
echo Core\Markdown::toHtml(
    $text,
    link: fn (string $url): ?string => "https://example.com" . $url,
    heading: fn (int $level, string $text): ?string => Core\Str::lower($text),
    code: fn (string $language, string $code): ?Core\Html\Markup =>
        Core\Html::escape("Run: " . Core\Str::trim($code)),
);
```
```output
<h2 id="md-install">Install</h2>
<p>See <a href="https://example.com/guide">the guide</a>.</p>
Run: nvs run app.nvs
```

## Converting a large document one part at a time

`Core\Markdown::toHtmlEach` converts a document in parts. Each part is a `Core\Html\Markup` with one
or more whole blocks, such as a paragraph, a heading or a list. A `foreach` loop reads the parts. The
parts joined together are the same HTML that `toHtml` returns for the whole document.

The document can be a `bytes` value, an open file, an upload part, a request body stream, or a list
of `bytes` chunks. A file is read twice. The first read finds the link definitions, because a link
can use a definition from the end of the file. Only one part is in memory at a time, so a file of any
size works. Every other source is kept in memory until the loop ends.

`toHtmlEach` takes the same four functions as `toHtml`. Each one runs when the loop reaches its part.
If the bytes are not valid UTF-8, the loop throws a `ParseError` when it reaches them.

```nvs
<?nvs
bytes $text = "See [the guide][g].\n\n- one\n- two\n\n[g]: /guide\n" as bytes;
foreach (Core\Markdown::toHtmlEach($text) as Core\Html\Markup $part) {
    echo "part: ", $part;
}
```
```output
part: <p>See <a href="/guide">the guide</a>.</p>
part: <ul>
<li>one</li>
<li>two</li>
</ul>
```

## Converting text that arrives in pieces

`Core\Markdown::live` creates a `Core\Markdown\Live` for a document that arrives in pieces, such as the
answer of a language model. Call `push` with each piece. A piece can end anywhere, also in the middle
of a word. `push` returns a `Core\Markdown\Update` with two parts:

- `done()` is the HTML of the blocks that this piece finished. Add it after the HTML you already have.
  A block is finished when the first line of the next block is complete. Its HTML never changes after
  that.
- `open()` is the HTML of the rest of the text, as if the document ended there. Show it after the
  finished blocks, and replace it at the next `push`.

Call `finish` at the end. It returns the HTML of the last blocks and the footnotes. The `done()` parts
and the `finish()` result joined together are the same HTML that `toHtml` returns for the whole text.
There is one exception. A link can only use a definition that arrived before its block was finished.
Otherwise the link is written as text.

`live` takes the same four functions as `toHtml`. The `open()` part is converted again at each `push`,
so a function can run more than once for the same block.

```nvs
<?nvs
Core\Markdown\Live $live = Core\Markdown::live();
array<string> $pieces = ["# Ans", "wer\n\nThe first ", "step.\n\n- one\n"];
foreach ($pieces as string $piece) {
    Core\Markdown\Update $update = $live->push($piece);
    echo "done: ", $update->done(), "open: ", $update->open(), "\n";
}
echo "finish: ", $live->finish();
```
```output
done: open: <h1>Ans</h1>

done: open: <h1>Answer</h1>
<p>The first</p>

done: <h1>Answer</h1>
<p>The first step.</p>
open: <ul>
<li>one</li>
</ul>

finish: <ul>
<li>one</li>
</ul>
```

## Converting HTML to Markdown

`Core\Markdown::fromHtml` converts HTML to Markdown. It takes a string, a `Core\Html\Markup` or a
`Core\Xml\Node` that `Core\Html::parse` returned. The result is a `tainted string`, because the HTML
can come from a user.

The HTML is cleaned the same way `Core\Html::sanitize` cleans it, so a `<script>` and its text are
removed. Text in the HTML stays text. A character such as `#` or `*` gets a backslash, so it is not
read as Markdown syntax. A table that Markdown cannot write, such as one with `colspan`, stays HTML.

```nvs
<?nvs
echo Core\Markdown::fromHtml("<h2>Notes</h2><p>Read <a href=\"/guide\">the <em>guide</em></a>.</p><ul><li>one</li><li>two</li></ul>");
echo "--\n";
echo Core\Markdown::fromHtml("<p># not a heading, *not* emphasis</p><script>alert(1)</script>");
```
```output
## Notes

Read [the *guide*](/guide).

- one
- two
--
\# not a heading, \*not\* emphasis
```

`fromHtml` reads the whole document into memory, as `Core\Html::parse` does. To convert one part of a
page, such as its article, parse the page and pass that element.
