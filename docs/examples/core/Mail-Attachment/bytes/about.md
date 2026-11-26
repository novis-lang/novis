Returns the content of an attachment, as `tainted bytes`. For an attachment you send, these are
the bytes that the mail contains. For an attachment of a received message, the sender wrote them.

The result is `tainted bytes` for every attachment, also when it was made from a `string`. Tainted
means that the value comes from outside your program, so you must check it before you use it.

An attachment made from a `string` stores the string as UTF-8. So a letter such as `é` is two
bytes. `Core\Bytes::length` gives the size of the
content, and `Core\Encoding::toHex` shows each byte.

In a test, `Core\Test::sentMail()[0]->attachments()` returns the attachments of the first mail
that was sent. Compare `bytes()` with the content you expect, written as a string with `as bytes`.

**Good to know:** the content is in memory for as long as the attachment exists. A mail with a
large file uses about 1.4 times the file's size while it is sent.

**The examples below** check the size of a file before it is sent, show a string returned as
`bytes`, and test the content of a CSV file.
