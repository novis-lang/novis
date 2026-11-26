`attachments()` returns the files that were sent with a message. In a test, `Core\Test::sentMail()`
returns one `Core\Test\SentMail` for each message, and `attachments()` is one of its methods.

The result is a list of `Core\Mail\Attachment` values, in the order your program gave them to the
`attachments` option of `Core\Mail::send`. When the program gave no attachments, the list is empty.
The list also contains each inline image. An inline image is an image that the HTML body shows
through its `cid:` URL.

Each `Core\Mail\Attachment` has these methods:

- `name()` returns the file name.
- `type()` returns the media type, such as `text/csv`.
- `bytes()` returns the content of the file.
- `isInline()` returns `true` for an inline image.

Use `attachments()` to check that a mail has the right file, with the right content.

**The examples below** show an invoice with a CSV file, a newsletter with a logo inside the HTML, and
a notice with no files.
