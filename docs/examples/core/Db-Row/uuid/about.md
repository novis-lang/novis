Reads one column of a row as an id.

`Core\Db\Row::uuid` takes the name of a column and returns a `Core\Uuid`. That is a 128-bit
identifier, the kind a table carries instead of a counted number. The column has to be declared as
one: `UUID` in PostgreSQL and in MariaDB 10.7 and later, `uniqueidentifier` in SQL Server, and
`uuid` in SQLite.

The result is `null` when the column has no value in this row, so the type you get back is
`?Core\Uuid`. Test it for `null` before you use it.

The method throws a `LogicError` when the row has no column with that name, and when the column is
not one of those types. A text column holding the same thirty-six characters is still text, and is
read with `Core\Db\Row::string`. MySQL stores an id as `BINARY(16)`, which is read with
`Core\Db\Row::bytes`.

**The examples below** read the id of an order, read a column that may have no value, and look up
rows by an id that arrived as text.
