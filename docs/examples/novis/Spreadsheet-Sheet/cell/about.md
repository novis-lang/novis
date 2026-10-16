Returns the address of one cell, such as `"C7"`, so a formula can use it.

`$key` is the key of a column, such as `"qty"`, or a column letter, such as `"C"`. `$row` is the row
number, and the first row of a sheet is row 1. The header row is row 1 when the rows have keys.

**Good to know:** a key is known after the first row with keys, or when the column is set in the
`columns` of `Workbook::sheet`. Before that, `cell` reads the key as column letters, so `"qty"` is
the column `QTY`. A row number below 1, or a column after the last column of a sheet, throws a
`LogicError`.

**The examples below** find the address of a cell, use two addresses in a formula, and show the error
for row 0.
