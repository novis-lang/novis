`Core\Test::sentMail()` returns every email your program sent during the current test. Under
`nvs test`, `Core\Mail::send` sends nothing and opens no connection, so a test never sends mail to a
real person.

In a test, `Core\Mail::send` still checks the `mail.send` capability, the `[mail.<name>]` block and
every address. If one of them is wrong, it throws the same error as in production. If all are
correct, the message is kept, and `sentMail()` returns it. The list includes the mail of the test,
of its `Core\Test::request` requests and of its `Core\Test::run` runs. It is in the order sent. Each
test starts with an empty list.

Each item is a `Core\Test\SentMail`. Its methods return the values given to `Core\Mail::send`:
`endpoint()`, `to()`, `subject()`, `text()`, `cc()`, `bcc()`, `replyTo()` and `html()`.

**Good to know:** outside `nvs test`, `sentMail()` returns an empty list, and `Core\Mail::send`
really sends the message.

**The examples below** show a welcome mail after sign-up, a password reset mail that a request
sends, and a contact form that sends no mail when it has an error.
