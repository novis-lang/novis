Reads one column of a row as a floating-point number.

`Core\Db\Row::float` takes the name of a column and returns the number in it. It reads the
floating-point column types: `REAL`, `FLOAT` and `DOUBLE PRECISION`. These columns store a number in
binary. A value like `0.1` has no exact binary form, so the column keeps the closest binary number
to it.

The result is `null` when the column has no value in this row, so the type you get back is `?float`.
Test it for `null`, or write `?? 0.0` after the call to use zero instead.

The method throws a `LogicError` when the row has no column with that name, and when the column is
not a floating-point column. A `DECIMAL` column holds an exact number, and reading it in binary
would change the value, so that column is read with `Core\Db\Row::decimal`. A whole-number column
is read with `Core\Db\Row::int` or `Core\Db\Row::uint`.

**The examples below** read a temperature, read a column that may have no value, and pick out the
stops inside a map area.
