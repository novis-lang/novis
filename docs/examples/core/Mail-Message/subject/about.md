Returns the subject of a received message, as a string.

A mail program writes a subject with letters outside ASCII in an encoded form, such as
`=?UTF-8?B?...?=`. `subject` decodes it, so the result is the text that the sender typed. A subject
that is written over several lines is joined into one line. A message without a `Subject` header
returns an empty string.

**Good to know:** the subject is tainted, because the sender wrote it. It can contain HTML, so escape
it before you put it on a page.

**The examples below** read an encoded subject, find an order number in a subject, and show a
subject on a web page.
