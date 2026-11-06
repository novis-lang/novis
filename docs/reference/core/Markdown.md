---
summary: Markdown to HTML that is safe to put into a page — CommonMark with tables, task lists, strikethrough, autolinks and footnotes
keywords: markdown, commonmark, gfm, github flavored markdown, readme, render, to html, comments, user content, xss
---

`Core\Markdown::toHtml` converts a Markdown document to HTML. It reads CommonMark with the GitHub
extensions: tables, task lists, strikethrough, autolinks and footnotes. The result is a
`Core\Html\Markup`, so you can write it into a page without escaping it again.

The result is safe even when a user wrote the document. HTML inside the document is cleaned the same
way `Core\Html::sanitize` cleans it. A link with an unsafe URL, such as `javascript:`, is written as
plain text. An image with an unsafe URL is removed. Any text is a valid document, so the method never
throws an error.

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
