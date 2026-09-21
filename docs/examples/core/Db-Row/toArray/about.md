Reads a whole row at once, as an array.

`Core\Db\Row::toArray` returns every column of the row in one array. The key of each entry is the
name of the column, and the value is what that column holds. The entries come in the order the query
asked for the columns.

A column with no value in this row is present and holds `null`. A name that is no column of the row
is not in the array.

The array is the row's own, so reading from it costs nothing. Writing to it gives your program its
own copy, and the row keeps what it read from the database.

Where you know the columns, a typed reader such as `Core\Db\Row::int` is the better choice. Use
`toArray` in a program that passes the whole row on.

**The examples below** read a whole row at once, print a row whose columns the program does not
know, and answer a request with rows as JSON.
