Says whether a column may hold SQL NULL, which means the column has no value in that row.

A query result describes its columns before you read any rows. `isNullable()` returns `true` when
the column may be empty, and `false` when it may not.

Every database driver returns `true` for every column. A query result does not include the
`not null` setting of the table. Finding it needs a second query on the table, and the driver does
not run one. So your program should treat every column as one that may be empty.

**In plain words:** read every value with a `?` type, such as `?string` or `?int`, and choose what
to show when there is nothing there.

**The examples below** print the description of a query first. Then they read a column that may be
empty, and count the empty values in a table.
