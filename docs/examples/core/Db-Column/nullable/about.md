Says whether a column may hold SQL NULL, which means the column has no value in that row.

A query result describes its columns before you read any rows. `nullable()` returns `true` when the
column may be empty, and `false` when it may not.

Today every database driver returns `true` for every column. A query result does not carry the
`not null` information from the table, and finding it needs a second question about the table
itself, which this driver does not ask. So your program should treat every column as one that may be
empty.

**In plain words:** read every value with a `?` type, such as `?string` or `?int`, and choose what to
show when there is nothing there.

**The examples below** print the description of a query first, then read a column that may be empty,
then count the empty values in a table.
