`bcc()` returns the list of addresses that get a hidden copy of a message. In a test,
`Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message, and `bcc()` is one of
its methods.

The list contains the `bcc` addresses that your program gave to `Core\Mail::send`, in the same
order. When the program gave no `bcc` addresses, the list is empty. A person who gets the message
cannot see the `bcc` addresses. The test can see all of them.

Use `bcc()` to check that addresses which must stay private are not in `to()` or `cc()`.

**The examples below** show a newsletter whose readers are all in `bcc`, an order mail with a
hidden copy for an archive, and invitations that a request sends.
