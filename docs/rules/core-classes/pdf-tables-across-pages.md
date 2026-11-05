A table that runs over several pages follows **eight rules, each a default with no configuration**:

1. **Column widths are computed once for the whole table**, so every page has the same columns. A
   table without a `width` is as wide as its columns' widest content, up to its containing block.
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
   rows when they do not. A `rowspan` group taller than a page splits between lines, as a tall row
   does.
7. **The caption shows on the first page only**, and `:nth-child` striping follows source order.
8. **A `<slot name="…">` in a repeated row** is filled per page by `Document::slots`
   (`rule:core-classes/pdf-header-is-a-page-function`). The slot keeps its row's height, so filling it
   never moves anything, and text that does not fit is listed in `Rendered::overflows`.

They are the rules an invoice, a statement and a long report need, and browsers keep only the second
half of the second: a table cut through a line of text, a lone header at a page bottom, an open border
and a carried-forward total that cannot be written are each a reason a program reaches for a second
tool. Each rule is the reading a person expects, so none needs a setting.

- **Memory:** the table's column widths and each row's height, once, as part of the layout tree,
  and each slot's box once on each page its row is drawn on.

Inside a page, a table follows CSS 2.1. A cell's `vertical-align` takes `top`, `middle`, `bottom`
and `baseline`, which aligns each cell's first line. A row's and a group's background is drawn
under its cells, and their borders only under `border-collapse: collapse`. A `display: table`
element directly in a table, a group or a row is wrapped in an anonymous cell. Under
`border-collapse: collapse` each edge is one border, resolved by CSS 2.1 §17.6.2.1 from every
box on it: `hidden` wins, then the wider border, then `double`, `solid`, `dashed`, `dotted`,
then the cell, row, row group, column, column group and table in that order. The edge at a page
break is resolved as the table's own edge, so rule 4 holds for the collapsed model too.

- **Memory:** nothing more. A `vertical-align` other than `top` and a group's box each cost one
  more layout pass over the cell or the box, freed when the table is placed. A collapsed table
  holds one grid slot per row and column and each row's resolved edges while it is laid out.

**On disk.** `extensions/pdf/src/layout/table.rs` lays a table out by the eight rules, and its
module doc names what it does not do yet. A closed edge takes its border and padding from each
page it is drawn on. A `col` or `colgroup` width, with `span`, sets its columns' widths; a
`colspan`, `rowspan` or `span` is read by HTML's rules for non-negative integers, so `2x` is 2,
and `rowspan="0"` runs to the end of its row group; a forced `break-before` or `break-after` on
a body row or a row group starts a new page between two pieces; the header and footer rows are
left off where no row fits with them; and a `direction: rtl` table and flex row run from the
right (`extensions/pdf/tests/tables.rs`). Collapsed borders resolve one border per edge, at a
page break and under a repeated header too
(`collapsed_borders_resolve_by_width_then_style_then_owner`).

**Declined:**
- **A setting per rule.** Each default is the one a document wants; `data-repeat`,
  `box-decoration-break` and `orphans` already override where a document wants otherwise.
- **Running totals in the table engine.** `data-mark` and a slot write one, with the arithmetic in
  Novis.
