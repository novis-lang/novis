Reads a raw email message, with its headers and its body, into a `Core\Mail\Message`.

You pass the whole message as a `string` or as `bytes`, such as an `.eml` file or the raw message a
mail provider posts to your webhook. The other methods then return its sender, recipients, subject,
text and attachments.

A broken message is not an error. When a header or a body is missing, `messageId()`, `from()`,
`date()`, `inReplyTo()`, `text()` and `html()` return `null`. The methods that return an array
return an empty array. `subject()` returns an empty string.

A part is one piece of a message, such as the text, the HTML or a file. A part can contain other parts. Novis
reads 16 levels of parts inside parts, and the first 1,000 parts. A part that cannot be read is
skipped.

**Good to know:** every value that the message returns is tainted. Tainted means that the value
comes from outside your program, so you must check or escape it before you use it.

**The examples below** read the message that a webhook posts, read an `.eml` file, and read a list
of messages that has a broken one in it.
