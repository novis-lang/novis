Sets a function that fills each `<slot name="...">` in a table, once for each page.

A table that runs over several pages repeats its `thead` and `tfoot` rows on each page. A cell in
one of those rows can hold `<slot name="carried"></slot>`. `render` calls the function once for each
page, with that page as a `Page`. The function returns an array from slot name to an `html` template,
and `render` draws each one in the cell of its slot on that page.

Use it for a total carried forward, or a subtotal for each page. `$page->marksUpTo("amount")`
returns the `data-mark="amount:..."` values up to this page, and `$page->marks("amount")` those on
this page only.

The HTML uses its own styles, not the document's, and starts at the top-left corner of the cell. A
`<style>` in the HTML can set its font size.

The slot keeps the height of its row, so filling it never moves anything on the page. HTML too tall
for its cell is cut off, and `Rendered::overflows` lists it with the slot name.

A slot's cell is as wide as its column, and a table without a `width` is as wide as its content.
Give the table a `width`, such as `width: 100%`, so the column has room for the HTML.

**The examples below** carry a total forward in a table, add up the hours of each page, and find a
fill that does not fit.
