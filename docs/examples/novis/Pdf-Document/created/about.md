Sets the creation time that the PDF file shows.

A PDF viewer shows this time in the properties of the file. A render reads no clock, so without
`created` the file has no creation time. Pass `Core\Time::now()` for the time of the render, or the
time of the order or the invoice the file is about.

A PDF/A file must have a creation time. With a PDF/A standard and no `created`, `render` throws a
`LogicError`.

**Good to know:** the same document with the same creation time always gives the same bytes. A test
can compare two files, and a cache can use the bytes as a key.

**The examples below** write the time into an invoice, show that the same time gives the same file,
and show that PDF/A needs a creation time.
