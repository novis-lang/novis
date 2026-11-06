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
