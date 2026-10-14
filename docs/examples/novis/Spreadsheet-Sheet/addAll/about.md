Adds each row of `$rows` to the end of the sheet, in order, and returns the sheet.

Each row is added the way `add` adds it. A row with keys puts each value in the column of its key,
and a row without keys puts the values in the columns in order, from column `A`.

`$rows` can be an array of rows or a generator. A generator gives one row at a time, so a long export
does not need all of its rows in memory at once.

**Good to know:** `addAll` gives no options to a row. To set the height, the style or `hidden` of one
row, add that row with `add`. When the workbook starts the next sheet, this sheet is finished, and
`addAll` throws a `LogicError`.

**The examples below** add an array of rows, add the rows of a generator, and add a styled header
row before the other rows.
