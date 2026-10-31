Sets a function that fills each `<slot name="...">` in a table, once for each page.

A table over several pages repeats its `thead` and `tfoot` rows on each page. A cell in those rows
can contain a `<slot>`. `render` calls your function once for each `Page`. The function returns an
array that maps each slot name to an `html` template.

`$page->marksUpTo("amount")` returns the `data-mark="amount:..."` values up to this page.
`$page->marks("amount")` returns those on this page only.

A fill does not use the document's styles, so set its font size with a `<style>`. A slot never
changes the height of its row. HTML that is too tall is cut off and listed in `Rendered::overflows`.

A table without a `width` is as wide as its content. Give it one, such as `width: 100%`, so the
slot's column has room.

**The examples below** carry a total forward, add up the hours of each page, and find a fill that
does not fit.
