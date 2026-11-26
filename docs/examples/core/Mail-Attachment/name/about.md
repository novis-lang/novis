Returns the file name of an attachment, such as `invoice.pdf`. This is the name the recipient sees
in the mail program.

An attachment made with `fromFile` or `inlineFile` has the last part of the path as its name.
`fromFile` also has a `name` option that gives another name. For `fromBytes` and `inlineBytes`, it
is the name you gave. A name
with letters that are not ASCII is returned as you wrote it.

`name()` returns a `tainted string`. Tainted means that the value comes from outside your program,
so you must check or escape it before you use it.

An attachment of a received message can have no file name, and then the result is `null`. If you
pass that attachment to `Core\Mail::send`, it throws a `RuntimeError`. Make a copy with
`fromBytes`, which gives it a name.

In a test, `Core\Test::sentMail()[0]->attachments()` returns the attachments of the first mail
that was sent.

**The examples below** show the name from a path, a list of names in the text of a mail, and a
test that checks the name.
