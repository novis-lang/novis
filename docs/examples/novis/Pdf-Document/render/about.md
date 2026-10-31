Makes the PDF file of a document, and returns it with its page count.

`render` returns a `Rendered`. `bytes` is the PDF file, which a program sends in a response, saves
or attaches to an email. `pageCount` is the number of pages.

The render also lists what it could not draw. `dropped` is the CSS it does not support, and
`missingGlyphs` is the text that no font has. `overflows` is a header, a footer or a slot that did
not fit. A render does not throw for these, so check the lists in a test.

A render reads no file, no network and no clock. The same document always gives the same bytes. A
very large document is limited by the memory and the time that one request may use. Tables, flex
containers and grid containers may be nested 32 levels deep. A deeper document throws a
`LogicError`.

**The examples below** make a shipping label with a barcode, list what a render could not draw, and
put a contract and its terms into one file.
