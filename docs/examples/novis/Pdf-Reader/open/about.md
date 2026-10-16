Opens an existing PDF file and reads its page count, its metadata and its attached files.

`Reader::open` takes the bytes of a PDF file and returns a `Reader`. `pageCount` is the number of
pages. `metadata` has the `title`, `author`, `subject`, `keywords`, `creator` and `producer` of the
file. Each of them is `null` when the file does not have it. The methods `attachments`, `invoiceXml`
and `text` read the rest of the file.

A file with a password throws a `ParseError`. Bytes that are not a PDF file also throw a
`ParseError`. A file that needs more memory than the PDF engine may use stops the program.

**Good to know:** `Reader::open` reads the attached files at once. The text of a page is read only
when you call `text`.

**The examples below** read the title of an upload, show the error for an encrypted file, and check
the page count of each upload.
