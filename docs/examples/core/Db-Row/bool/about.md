Reads one column of a row as a truth value.

`Core\Db\Row::bool` takes the name of a column and returns `true` or `false`. The column is a boolean
column, or a whole number column that holds `0` or `1`. MySQL and MariaDB have no boolean column of
their own, so a `TINYINT(1)` column there is read with this method too.

The result is `null` when the column has no value in this row, so the type you get back is `?bool`.
Write `?? false` to choose what your program uses instead.

The method throws a `LogicError` when the row has no column with that name, when the column holds
text, a number with a fraction or a date, and when a whole number column holds any other number. A
stored `7` is an error, not `true`.

**The examples below** read a yes or no column, read a column that may have no value, and read a flag
that is stored as `0` or `1`.
