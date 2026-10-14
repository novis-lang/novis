Returns the chart with one more series. `$values` is the range of the values, such as `B2:B10`.
The chart you call it on does not change, so keep the chart that `series` returns.

Every option is optional. `categories` is the range of the labels under the values, or the x
values of a scatter chart. `name` is the name of the series in the legend. `color` is a color such
as `'#1F77B4'`.

**Good to know:** use `$sheet->column($key)` for each range. That range grows with the sheet, so
the chart shows every row. A range of another sheet starts with the sheet name, such as
`Sales!B2:B10`. `Sheet::chart` checks each range and color, and throws a `LogicError` for one that
is not correct.

**The examples below** add one series for each shop, use the rows of a sheet whose name has a
space, and show the errors for a range and a color that are not correct.
