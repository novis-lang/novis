`text()` returns the plain-text body of a message. In a test, `Core\Test::sentMail()` returns one
`Core\Test\SentMail` for each message, and `text()` is one of its methods.

When your program gave `Core\Mail::send` a `string` body, `text()` returns that string, with every
line break in it. When the body was HTML, `text()` returns the plain text made from it, or the
`text` option when the program gave one. Use `html()` to read the HTML body.

Use `text()` to check what a person reads in the message. The `Core\Str` functions help you check
one part of it, such as a link or one line, without writing out the whole body.

**The examples below** show a body with a confirmation link, a receipt with one line for each item,
and a sign-up form whose mail must not contain the password from the form.
