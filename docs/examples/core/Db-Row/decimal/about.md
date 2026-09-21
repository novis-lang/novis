Reads one column of a row as an exact number.

`Core\Db\Row::decimal` takes the name of a column and returns a `decimal`. That is a number with up
to 29 digits, and it keeps every one of them. It is the type for money. The columns it reads are
`DECIMAL`, `NUMERIC` and `MONEY`.

The result is `null` when the column has no value in this row, so the type you get back is
`?decimal`. Test it for `null` before you use it.

The method throws a `LogicError` when the row has no column with that name, and when the column is
not an exact numeric one. A `FLOAT`, `REAL` or `DOUBLE` column is read with `Core\Db\Row::float`,
and that method reads no `DECIMAL` column. A price that passes through a `float` can come back a
hundredth away from the value that was stored.

**The examples below** print the price of a product, read a column that may have no value, and add
up what an order costs.
