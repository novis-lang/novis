Opens an existing PDF file and reads its page count, its metadata and its attached files.

`Reader::open` takes the bytes of a PDF file and returns a `Reader`. `pageCount` is the number of
pages. `metadata` has the `title`, `author`, `subject`, `keywords`, `creator` and `producer` of the
file. Each of them is `null` when the file does not have it. The methods `attachments`, `invoiceXml`
and `text` read the rest of the file.

A file with a password throws a `ParseError`. Bytes that are not a PDF file also throw a
`ParseError`. Parts of a PDF file are often compressed. A part that unpacks to more than 64 MB,
or to more than 1000 times its packed size, throws a `ParseError`. The attached files together may
also unpack to at most 64 MB. The server's `[limits] max_decompressed` and
`[limits] max_decompression_ratio` set these two numbers.

**Good to know:** `Reader::open` reads the attached files at once. The text of a page is read only
when you call `text`.

**The examples below** read the title of an upload, show the error for an encrypted file, and check
the page count of each upload.
