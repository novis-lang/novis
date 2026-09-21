Runs one statement that reads rows, and returns all of them.

`Core\Db\Connection::query` is the method for a `select`. It returns a `Core\Db\Rows` object. A
`foreach` over that object reads one row at a time. `all()` gives every row as an array, `count()`
gives the number of rows, and `first()` gives the first row or `null`.

Every row is read into memory before the method returns. The connection is free again right after
that, so the next statement can run while you are still reading the rows. When a result is too
large to hold in memory, use `Core\Db\Connection::stream` instead.

A value never goes into the text of the statement. Write a `?` for each value, or a `:name` for
each, and pass the values as the second argument. The database keeps the statement and the values
apart, so a value that looks like SQL stays a value.

The examples read the rows one at a time, read a result without a loop, and build a report from a
statement whose values are named.
