Returns the file name of an attachment, such as `invoice.pdf`. This is the name the recipient sees
in the mail program.

For `fromFile`, the name is the last part of the path, unless the `name` option gives another one.
For `inlineFile`, it is also the last part of the path. For `fromBytes` and `inlineBytes`, it is
the name you gave. A name with letters that are not ASCII is returned as you wrote it. The mail encodes it when it is
sent.

`name()` returns a `tainted string`. Tainted means that the value can come from a user, such as a
file name from an upload form. Escape it before you put it into HTML.

In a test, `Core\Test::sentMail()[0]->attachments()` returns the attachments of the first mail
that was sent. `name()` then checks which files the mail had.

**The examples below** show the name from a path, a list of names in the text of a mail, and a
test that checks the name.
