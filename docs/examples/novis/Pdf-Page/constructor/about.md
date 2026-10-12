Makes one page of a PDF document, with its number, the page count and the values a header or footer needs.

`Document::render` makes a `Page` for each page and passes it to your header, footer or slots function.
`number` is the page number, counted from 1. `count` is the number of pages in the document. `name` is
the named page that CSS `page` gives it, or `null`.

You can also write `new Page` yourself. A test can then call a header function with any page it
wants, without a render. The last two values are the `string-set` values of this page and the marks
of every page. Each is a list of names and values.

**The examples below** test a footer function with a page made by hand, print the number and count
of each page in a render, and show a named cover page.
