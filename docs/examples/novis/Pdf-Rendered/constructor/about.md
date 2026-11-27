The result of `Document::render`: the PDF file, its pages, and the lists of what the render could not draw.

`bytes` is the PDF file. `pageCount` is the number of pages. `pages` has one `Page` for each page,
with the marks and `string-set` values of that page. `dropped` lists each CSS declaration
the render does not support, with its selector, line and column. `missingGlyphs` lists text that no
font can draw, with its page. `overflows` lists header, footer or slot content that is too tall for
its area. A render that worked well has three empty lists.

**Good to know:** a test can check that `dropped` is empty. The test then fails when somebody adds
CSS that the render ignores. You can also write `new Rendered` yourself, for example in a test.
With `pages`, you can find the page of each mark after the render. To print a page number in the same document,
such as in a table of contents, use the CSS function `target-counter()` instead. It needs only one
render.

**The examples below** read the file and the page of each order, show the three lists, and make a
result by hand for a test.
