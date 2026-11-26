`text()` returns the plain text that `Core\Mail::send` would send for the body of a
`Core\Mail\Preview`. `Core\Mail::preview` returns the preview.

Every mail has a plain text. A mail program that does not show HTML shows it. It is one of three
things:

- For a `string` body, it is the body exactly.
- For a `Core\Html\Markup` body with the `text` option, it is the `text` option exactly.
- For a `Core\Html\Markup` body without the option, it is made from the HTML.

When it is made from the HTML, a heading or a paragraph starts a new block after an empty line. A
`<br>` is a line break, and a list item is a line that starts with `- `. A link is its text and
then its URL in brackets. An image is its `alt` text, and the cells of a table are separated by a
tab. `<style>`, `<script>` and `<head>` give no text. A line is wrapped at 78 characters.

**The examples below** show the plain text of an HTML mail, a plain text written by hand, and a
test of a link.
