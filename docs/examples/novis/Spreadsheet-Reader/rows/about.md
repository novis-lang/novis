Reads the rows of one sheet, one `Row` at a time.

`$sheet` is the sheet name, or its position from 0. Each `Row` has `number`, the row number in the
file, and `values`. Without options, the keys of `values` are the column letters, such as `A`. With
`header: true`, the first row that is not empty has the keys, and the rows after it are the data.
`header: 2` says the keys are in row 2. `from` is the first row to read, `limit` is the most rows, and
`columns` reads only some columns, such as `['A', 'D']`.

**Good to know:** empty rows are skipped. Use `emptyRows: 'keep'` to get them. A row of formulas
with no saved result is not empty. Each of its values is `null`, and with `formulas: true` each is a
`Formula` whose `cached` is `null`. With `merged: 'fill'` and `text`, each cell of a merged range
has the text of its top-left cell. A string from the
file is `tainted`, so check it before you trust it. `text: ['Zip']` reads the column `Zip` as the text
Excel shows, so a zip code keeps its leading zero. The rows are read in batches, so all the rows of a
large sheet are never in memory at the same time.

**The examples below** import an upload and report each bad row by its row number, read a zip code
column as text, and read some rows and columns of a large sheet.
