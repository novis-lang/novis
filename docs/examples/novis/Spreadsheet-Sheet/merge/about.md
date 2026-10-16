Joins the cells of `$range`, such as `"A1:C1"`, into one cell, and returns the sheet.

Excel shows the value of the top left cell over the whole range. The range can use rows you already
added and rows you add later, until the sheet is finished.

**Good to know:** two merged ranges of one sheet cannot overlap, and a second range that overlaps the
first throws a `LogicError`. A text that is not a range of cells, such as `"A1"` alone or `"title"`,
throws a `LogicError` too. A CSV file has no merged cells, so `merge` does nothing there.

**The examples below** put a title over three columns, merge cells of rows that are already added,
and show the error for two ranges that overlap.
