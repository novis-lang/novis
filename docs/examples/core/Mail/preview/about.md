Returns the body of a message exactly as `Core\Mail::send` would send it, and sends nothing.

`preview` takes the same body as `send`, and its `text` and `attachments` options. It needs no
`[mail.<name>]` block. The result is a `Core\Mail\Preview` with four methods:

- `html()` returns the HTML with its CSS copied into the elements, or `null` for a `string` body.
- `text()` returns the plain text.
- `keptCss()` returns the CSS rules that stay in a `<style>` element.
- `unusedCss()` returns the rules that match no element.

Use it to show a mail on a page, or to check a mail in a test.

Each CSS rule of a `<style>` element is copied into the `style` attribute of each element it
matches. Most mail programs ignore a `<style>` element, so the mail looks the same everywhere. A
rule such as `@media` or `a:hover` stays in the `<style>` element.

A mistake in the body throws the same `RuntimeError` as `send`. For example, a
`<link rel="stylesheet">` throws, because a mail program does not load a stylesheet.

**The examples below** show a styled mail, a test of the plain text, and a stylesheet link.
