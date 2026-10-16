Reads one column of a row as a whole number.

`Core\Db\Row::int` takes the name of a column and returns the number in it. It reads the integer
column types: `SMALLINT`, `INT` and `BIGINT`. An unsigned column is read too, as long as its value
fits in an `int`.

The result is `null` when the column has no value in this row, so the type you get back is `?int`.
Test it for `null`, or write `?? 0` after the call to use zero instead.

The method throws a `LogicError` when the row has no column with that name, and when the column is
not an integer column. A text column holding the characters `7` is still text, and a column with a
fraction is read with `Core\Db\Row::float` or `Core\Db\Row::decimal`. It also throws for an
unsigned value above `9223372036854775807`, which is the largest number an `int` holds. Read that
column with `Core\Db\Row::uint`.

**The examples below** read the number of seats at a table, read a column that may have no value,
and compare two numbers out of the same row.
