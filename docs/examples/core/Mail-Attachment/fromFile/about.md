Reads a file from the disk and returns it as an attachment for `Core\Mail::send`.

Pass the result in the `attachments` option of `Core\Mail::send`. The file is read when
`fromFile` runs. So a missing file throws an `IOError` on the line that
names it. A file that the `fs.read` capability does not cover throws a `RuntimeError` on the same
line.

A relative path written in the code, such as `'files/invoice.pdf'`, starts at the folder of the
program file that contains it.

The recipient sees the last part of the path as the file name. The `name` option gives another
name. The type is a media type, such as `application/pdf`. It says what kind of file it is.
`fromFile` detects it from the first bytes of the file. With the `type` option, you set it yourself.

**Good to know:** the whole file is kept in memory until the mail is sent. For a large file, send a
download link instead.

**The examples below** send an invoice as a PDF, give a file a clearer name, and show the two
errors.
