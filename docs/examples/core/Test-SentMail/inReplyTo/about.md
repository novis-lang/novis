`inReplyTo()` returns the message id of the message that this message answers. In a test,
`Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message, and `inReplyTo()` is one
of its methods.

The result is the `inReplyTo` option that your program gave to `Core\Mail::send`. When the program
gave no `inReplyTo` option, the result is `null`. The id has no angle brackets. It is the same
string that `Core\Mail::send` returned for the first message.

Mail programs use this id to show a message and its answer in one thread. Most programs also read
the `references` option, so an answer usually sets both. `Core\Mail::send` throws an error when the
value is not a message id, such as a value with a space or a line break.

Use `inReplyTo()` to check that an answer belongs to the right thread.

**The examples below** show an answer to a support ticket, a new message that starts its own
thread, and a request that answers a question in the same thread.
