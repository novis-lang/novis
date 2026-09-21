Reads one column of a row as a whole number that is never negative.

`Core\Db\Row::uint` takes the name of a column and returns the number in it. It reads the unsigned
integer columns MySQL and MariaDB write as `… UNSIGNED`, and it reads a signed column too, as long
as the value in this row is not negative.

The result is `null` when the column has no value in this row, so the type you get back is `?uint`.
Test it for `null` before you use it.

The method throws a `LogicError` when the row has no column with that name, when the column is not
an integer column, and when the value is negative. A count, a size and a version number are never
negative, which is what this reader is for. Where a column may hold a negative number, read it with
`Core\Db\Row::int`.

**The examples below** read the size of a file, read a column that may have no value, and add up
the sizes in a folder.
