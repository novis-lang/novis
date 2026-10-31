Returns a value that CSS `string-set` saves on a page, such as the title of the current chapter.

In CSS, `h2 { string-set: chapter content() }` saves the text of each `h2` under the name `chapter`.
`string("chapter")` returns the value for this page. If no `h2` is on the page, the value is the
last one from an earlier page. If no value is set yet, the result is `null`.

The name must have the same case as in the CSS. For the name `chapter`, `string("Chapter")`
returns `null`.

**The examples below** put the chapter title in the header, show the `null` before the first
chapter, and put the customer name on each page of a batch of invoices.
