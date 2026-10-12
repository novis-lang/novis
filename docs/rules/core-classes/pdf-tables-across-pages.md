A table that runs over several pages follows **eight rules, each a default with no configuration**:

1. **Column widths are computed once for the whole table**, so every page has the same columns.
2. **`thead` repeats at the top and `tfoot` at the bottom of every page the table is on.** A row
   inside either says when it shows with `data-repeat`: `every` (the default), `first`, `last`, or
   `continued`, which shows only where the table goes on to another page.
3. **A row that fits on a page is never split.** A row taller than a page splits between lines of
   text, never through a line, an image or a glyph, and each cell continues on the next page.
4. **Borders are closed at every break**: the table's outer border and collapsed cell borders are
   drawn on both sides of it. `box-decoration-break: slice` on the table restores CSS's open edge.
5. **A header is never alone.** `orphans` and `widows` on a table count rows, default 2, and a table
   never starts at a page bottom with only its `thead`.
6. **A `<tbody>` and a `rowspan` group stay together** when they fit on a page, and split between
   rows when they do not.
7. **The caption shows on the first page only**, and `:nth-child` striping follows source order.
8. **A `<slot name="…">` in a repeated row** is filled per page by `Document::slots`
   (`rule:core-classes/pdf-header-is-a-page-function`). The slot keeps its row's height, so filling it
   never moves anything, and text that does not fit is listed in `Rendered::overflows`.

They are the rules an invoice, a statement and a long report need, and browsers keep only the second
half of the second: a table cut through a line of text, a lone header at a page bottom, an open border
and a carried-forward total that cannot be written are each a reason a program reaches for a second
tool. Each rule is the reading a person expects, so none needs a setting.

- **Memory:** the table's column widths and each row's height, once, as part of the layout tree.

**Not on disk.** `extensions/pdf/src/layout.rs` lays a `display: table` box out as a block box, so
none of the eight rules holds yet. The `pdf` goal's Stage 11 builds them.

**Declined:**
- **A setting per rule.** Each default is the one a document wants; `data-repeat`,
  `box-decoration-break` and `orphans` already override where a document wants otherwise.
- **Running totals in the table engine.** `data-mark` and a slot write one, with the arithmetic in
  Novis.
