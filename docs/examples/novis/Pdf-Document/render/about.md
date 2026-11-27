Makes the PDF file of a document, and returns it with its page count.

`render` returns a `Rendered`. `bytes` is the PDF file, which you can send, save or attach to an
email. `pageCount` is the number of pages.

The render also lists what it could not draw. `dropped` lists the CSS it does not support. It also
lists each name in `font-family` that has no font, such as a typo. A font must come from
`@font-face` or be a built-in font. `missingGlyphs` lists the text that no font has. `overflows`
lists each header, footer, slot or `@page` margin box that did not fit. A render does not throw an
error for these, so check the lists in a test.

A render reads no file, no network and no clock. The same document always gives the same bytes. A
render has the memory and time limits of its request. Tables, flex containers and grid containers
may be nested 32 levels deep. A deeper document throws a `LogicError`.

**The examples below** make a shipping label with a barcode, list what a render could not draw, and
put a contract and its terms into one file.
