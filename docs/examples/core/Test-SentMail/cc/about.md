`cc()` returns the list of addresses that get a copy of a message. In a test,
`Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message, and `cc()` is one of its
methods.

The list contains the `cc` addresses that your program gave to `Core\Mail::send`, in the same
order. When the program gave no `cc` addresses, the list is empty. Every person who gets the
message can see the `cc` addresses.

Use `cc()` to check that the right people get a copy, and that nobody else does.

**The examples below** show an invoice with a copy for the accounts team, a shipping notice with
no copy, and a refund request that a request sends with a copy for a manager.
