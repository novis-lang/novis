Returns the address of one cell, such as `"C7"`, so a formula can use it.

`$key` is the key of a column, such as `"qty"`, or a column letter, such as `"C"`. A column letter is
upper case, from `"A"` to `"XFD"`. Any other text is a key, so `"c"` is a key. `$row` is the row
number, and the first row of a sheet is row 1. The header row is row 1 when the rows have keys.

**Good to know:** a key is known after the first row with keys, or when the column is set in the
`columns` of `Workbook::sheet`. Before that, a key throws a `LogicError` that names the key. A row
number below 1, or a column after the last column of a sheet, throws a `LogicError`. When the
workbook starts the next sheet, this sheet is finished. Then `cell` throws a `LogicError` that names
the sheet.

**The examples below** find the address of a cell, use two addresses in a formula, and show the error
for row 0.
