Returns the whole content of one uploaded file as a single `tainted bytes` value. Use it for a file
that is small enough to keep in memory, such as a picture, an invoice or a short document. An empty
file gives empty bytes.

Without an argument, the file may be as large as the `[limits] request_body` setting, which is 8 MB
by default. `{max: …}` sets your own largest size in bytes. A file larger than the limit throws a
`RuntimeError`. A `max` larger than the `[limits] memory` setting of the request throws a
`LogicError`. Call `readAll` while the `Core\Request::files()` loop is on this file. After the loop
moves on, it throws a `LogicError`. The content can be read only once, so a second call returns empty
bytes.

Use `content()` to read a large file in pieces, and `saveTo` to write it to a file.

The examples show how to print a small text file, how to set a size limit, and how to check that an
upload really is a PDF file.
