Makes the cells of `$range`, such as `"A1:C9"`, an Excel table, and returns the sheet. The first row
of the range is the header row of the table, and each header needs its own name.

`name` is the name of the table. `style` is the name of an Excel table style. `banded` colours every
second row, and `filter` shows a filter button in each header. Both are `true` unless you set them to
`false`. `totals` adds a totals row after the table. It maps a header to `"sum"`, `"average"`,
`"count"`, `"min"`, `"max"` or a `Formula`.

**Good to know:** the totals row is the row after the table. When you add the table after its last
row, the totals row is written at once, and your next row comes after it. Any other total throws a
`LogicError`. A CSV file has no tables, so `table` does nothing there.

**The examples below** make a table of orders, add a totals row, and show the error for a total that
is not allowed.
