`messageId()` returns the message id of a message. In a test, `Core\Test::sentMail()` returns one
`Core\Test\SentMail` for each message, and `messageId()` is one of its methods.

`Core\Mail::send` writes a new message id into every message, and it returns that id. `messageId()`
returns the same string. It has no angle brackets. It looks like `3f2a…@example.com`: 32 random hex
digits, `@`, and the domain of the `from` address of the mail block.

A program stores the id when it needs to find the message again. A reply to the message contains this
id, and so does a bounce. Pass the id to the `inReplyTo` option to send a reply in the same thread.

Use `messageId()` to check that your program stored the id of the message it sent.

**The examples below** show an order that keeps the id of its confirmation, two messages with
different ids, and a receipt that a request sends after its response.
