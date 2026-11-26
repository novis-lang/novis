`references()` returns the message ids of the earlier messages in a thread. In a test,
`Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message, and `references()` is
one of its methods.

The result is the `references` option that your program gave to `Core\Mail::send`, in the same
order. The oldest message comes first. When the program gave no `references` option, the list is
empty. Each id has no angle brackets, in the same form that `Core\Mail::send` returns.

Mail programs use this list to put a message in the right thread, even when a message in the
middle is missing. An answer usually sets `inReplyTo` to the last id of the list. `Core\Mail::send`
throws an error when an item of the list is not a message id.

Use `references()` to check that a new message lists the whole thread.

**The examples below** show a thread of three messages, a password reset that is not in a thread,
and a comment that a request adds to a thread.
