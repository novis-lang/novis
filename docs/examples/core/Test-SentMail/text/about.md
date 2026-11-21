`text()` returns the plain-text body of a message. In a test, `Core\Test::sentMail()` returns one
`Core\Test\SentMail` for each message, and `text()` is one of its methods.

The body is the `$text` string that your program gave to `Core\Mail::send`, with every line break
in it. A message that also has an HTML body still has this plain-text body. Use `html()` to read
the HTML body.

Use `text()` to check what a person reads in the message. The `Core\Str` functions help you check
one part of it, such as a link or one line, without writing out the whole body.

**The examples below** show a body with a confirmation link, a receipt with one line for each item,
and a sign-up form whose mail must not contain the password from the form.
