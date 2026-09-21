Reads one column of a row as text.

`Core\Db\Row::string` takes the name of a column and returns the text in it. It reads the text
family of column types and nothing else: `CHAR`, `VARCHAR`, `TEXT`, `ENUM` and `JSON`.

The result is `null` when the column has no value in this row, so the type you get back is
`?string`. Test it for `null`, or write `?? ''` after the call to use an empty text instead.

The method throws a `LogicError` when the row has no column with that name, and when the column is
not one of those types. A number is not written out as text here. A `BINARY`, `BLOB` or `BYTEA`
column is read with `Core\Db\Row::bytes`, and a `UUID` column with `Core\Db\Row::uuid`, even though
an id looks like text when you print it.

**The examples below** read the name and the city of a person, read a column that may have no
value, and count the orders of each status.
