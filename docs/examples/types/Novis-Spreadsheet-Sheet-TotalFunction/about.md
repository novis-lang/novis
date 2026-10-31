The function that the totals row of a table uses for one column.

You set it with the option `totals` of `Sheet::table`. It maps a header to a string, and
`Sheet::table` turns the string into a `Sheet::TotalFunction` case:

- `'sum'` is `Sum`, the sum of the column.
- `'average'` is `Average`, the average of the column.
- `'count'` is `Count`, the number of cells with a value.
- `'min'` is `Min`, and `'max'` is `Max`.

Any other string throws a `LogicError`. For another calculation, map the header to a `Formula`.
Excel calculates each total when it opens the file.
