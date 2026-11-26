Returns the file name of an attachment, such as `invoice.pdf`. This is the name the recipient sees
in the mail program.

For `fromFile`, the name is the last part of the path, unless the `name` option gives another one.
For `inlineFile`, it is also the last part of the path. For `fromBytes` and `inlineBytes`, it is
the name you gave. A name with letters that are not ASCII is returned as you wrote it. `send`
encodes the name in the mail.

`name()` returns a `tainted string`, or `null` when the attachment has no name. Tainted means that
the value comes from outside your program, so you must check or escape it before you use it. Escape
it before you put it into HTML.

An attachment of a received message can have no file name, and then the result is `null`. If you
pass that attachment to `Core\Mail::send`, it throws a `RuntimeError`. Make a copy with
`fromBytes`, which gives it a name.

In a test, `Core\Test::sentMail()[0]->attachments()` returns the attachments of the first mail
that was sent. Compare each `name()` with the file name you expect.

**The examples below** show the name from a path, a list of names in the text of a mail, and a
test that checks the name.
