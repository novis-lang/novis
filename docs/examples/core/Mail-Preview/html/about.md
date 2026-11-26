`html()` returns the HTML that `Core\Mail::send` would send for the body of a `Core\Mail\Preview`.
`Core\Mail::preview` returns the preview.

Most mail programs ignore a `<style>` element. So each CSS rule of a `<style>` element is copied
into the `style` attribute of each element it matches. The order follows CSS: a more specific rule
wins, then the later rule. A value with `!important` wins over a normal value. The element's own
`style` attribute wins over a normal rule.

A rule that is copied is removed from the `<style>` element. An empty `<style>` element is removed.
A rule that cannot be copied stays, such as `@media` or `a:hover`. `keptCss()` lists those rules.

The result is a whole HTML document. A body that has no `<html>`, `<head>` or `<body>` element
gets them.

When the body is a `string`, the mail has no HTML, and `html()` returns `null`. Use `text()` for
the plain text.

**The examples below** show a button that gets its style, an element whose own `style` attribute
wins, and a plain-text mail with no HTML.
