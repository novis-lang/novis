Returns the plain text body of a received message, or `null` when the message has no plain text
part.

The text is decoded, so an encoded body, such as base64 or quoted-printable, is returned as normal
text. When the message has several plain text parts, `text()` returns the first one.

The sender wrote this text, so `text()` returns a `tainted string`. Tainted means that the value
comes from outside your program, so you must check or escape it before you use it. You can print
it, search it and split it. Before you put it on a page, escape it with `Core\Html::escape`.

A message with an HTML body only has no plain text part. `html()` returns its HTML body.

**The examples below** show the text of a reply, a message with HTML only, and a reply without its
quoted lines.
