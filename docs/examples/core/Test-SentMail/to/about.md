`to()` returns the list of addresses that a message was sent to. In a test, `Core\Test::sentMail()`
returns one `Core\Test\SentMail` for each message, and `to()` is one of its methods.

The list contains the `$to` addresses that your program gave to `Core\Mail::send`, in the same
order. The `cc` and `bcc` addresses are not in this list. Use `cc()` and `bcc()` to read them.

Use `to()` to check that a mail goes to the right person. This matters most for a mail with a
secret link in it, such as a password reset.

**The examples below** show an order confirmation for one customer, a message to a team of three
people, and a password reset mail that must go to the address of the account.
