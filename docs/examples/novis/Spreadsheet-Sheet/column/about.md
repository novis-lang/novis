Returns the range of one column in the rows added so far, such as `"C2:C1043"`, so a formula or a
chart can use it.

`$key` is the key of a column, such as `"qty"`, or an upper-case column letter, such as `"C"`. The
header row is not in the range, so the range starts at the first row of data.

**Good to know:** the range ends at the last row you added before the call. Call `column` after the
last row to get every row. A chart is the one exception: a series range from `column` grows with the
sheet, so a chart you add before the last row still shows every row. Before the first row of data, `column` throws a
`LogicError`. When the workbook starts the next sheet, this sheet is finished, and `column` throws a
`LogicError`.

**The examples below** find the range of a column, add a total row with `SUM`, and show how the
range grows as rows are added.
