Reads the rows of one sheet, one `Row` at a time.

`$sheet` is the sheet name, or its position from 0. Each `Row` has `number`, the row number in the
file, and `values`. The keys of `values` are the column letters, such as `A`. With `header: true`,
the first row that is not empty gives the keys instead. `header: 2` takes them from row 2. `from` is
the first row to read, `limit` is the most rows, and `columns` reads only some columns.

**Good to know:** empty rows are skipped. Use `emptyRows: 'keep'` to get them. A row of formulas
with no saved result is not empty, and each of its values is `null`. With `merged: 'fill'`, each
cell of a merged range has the value of its top-left cell. A string from the file is `tainted`, so
check it before you trust it. The rows are read in batches, so a large sheet is never all in memory.
