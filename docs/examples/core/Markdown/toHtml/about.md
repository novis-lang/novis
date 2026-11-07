Converts a Markdown document to HTML that is safe to write into a page.

The result is a `Core\Html\Markup`, so you can print it or put it into a template as it is. Any text
is a valid document, so `toHtml` never throws an error because of the text. It reads CommonMark with
tables, task lists, strikethrough, autolinks and footnotes.

The document may come from a user. HTML inside it is cleaned the same way `Core\Html::sanitize`
cleans it, so a `<script>` element or an `onclick` attribute is removed. A link with an unsafe URL,
such as `javascript:`, is written as plain text. An image with an unsafe URL is removed.

You can pass four optional functions by name. `link` and `image` change each URL, `heading` gives each
heading an `id`, and `code` writes the HTML of a code block, for example with syntax highlighting.

**Good to know:** the time `toHtml` needs grows in step with the length of the document. A document
written to be slow, such as thousands of open brackets, is still converted quickly.
