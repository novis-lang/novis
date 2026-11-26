Returns the media type of an attachment, such as `application/pdf` or `image/png`. The media type
says what kind of file it is. A mail program uses it to choose how to open the file.

When the attachment was made with the `type` option, `type()` returns that type in small letters.
Without the option, the type is detected from the first bytes of the content. A PDF file, an image
or a ZIP file is detected. Plain text has no such start, so a CSV file or a text file gets
`application/octet-stream`. Give the `type` option for those files.

The `type` option takes only the media type. A part after it, such as `; charset=utf-8`, throws a
`RuntimeError` when the attachment is made.

`type()` returns a `tainted string`, because the type can come from a user. In a test,
`Core\Test::sentMail()[0]->attachments()` returns the attachments of the first mail. Compare the
`type()` of each one with the type you expect.

**The examples below** show a detected type and a given one, the error for a type with a charset,
and a test that checks the types.
