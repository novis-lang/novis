Reads a raw email message, with its headers and its body, into a `Core\Mail\Message`.

You pass the whole message as a `string` or as `bytes`. This is the content of an `.eml` file, or the
raw message that a mail provider posts to your webhook. The other methods of the message then return
its sender, its recipients, its subject, its text and its attachments.

A broken message is not an error. When a header or a body is missing, `messageId()`, `from()`, `date()`,
`inReplyTo()`, `text()` and `html()` return `null`. `to()`, `cc()`, `replyTo()`, `references()`,
`header()` and `attachments()` return an empty array. `subject()` returns an empty string.

A message is made of parts. A part is one piece of a message, such as the text, the HTML or a
file. A part that cannot be read is skipped. So a program that reads many messages reaches all of
them.

**Good to know:** every value that the message returns is tainted. Tainted means that the value
comes from outside your program, so you must check or escape it before you use it. Escape a value
with `Core\Html::escape` before you put it on a page.

Novis reads the first 16 levels of nested parts and the first 1,000 parts, and skips the rest.

**The examples below** read the message that a webhook posts, read an `.eml` file, and read a list
of messages that has a broken one in it.
