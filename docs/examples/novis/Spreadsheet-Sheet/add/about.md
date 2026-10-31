Adds one row to the end of the sheet and returns the sheet, so the next call can follow it.

When the row has keys, each value goes to the column of its key, and the keys of the first row are
the header row. When the row has no keys, the values go to the columns in order, from column `A`. A
value can be text, a number, a date, a `Formula` or a `Cell`.

`$options` changes how the row looks. `height` is the row height in points. `hidden: true` hides the
row. `level` puts the row in an outline group. `style` is a `Style` for every cell of the row.

**Good to know:** the rows are written in order, and a row cannot be changed after you add it. When
the workbook starts the next sheet, this sheet is finished, and `add` throws a `LogicError`. A CSV file
has no heights, styles or hidden rows, so these options do nothing there. A number that is NaN or an
infinity throws a `LogicError` that names its cell, in every format.

**The examples below** add rows with keys, add rows without keys, and hide a row.
