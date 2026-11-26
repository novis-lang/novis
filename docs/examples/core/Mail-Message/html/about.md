Returns the HTML body of a received message, or `null` when the message has no HTML part.

The sender wrote this HTML, so `html()` returns a `tainted string`. Tainted means that the value
comes from outside your program. A tainted string cannot be put on a page as HTML. Pass it to
`Core\Html::sanitize` first. It keeps simple formatting, such as paragraphs, bold text and links,
and it removes scripts.

The HTML is decoded. An encoded body, such as quoted-printable, is returned as normal text.

Many messages have a plain text body as well. `text()` returns it.

**The examples below** show an HTML reply made safe for a page, a message with no HTML, and a
decoded HTML body.
