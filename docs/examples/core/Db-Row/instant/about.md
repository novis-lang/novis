Reads one column of a row as a moment in time.

`Core\Db\Row::instant` takes the name of a column and returns a `Core\Time\Instant`. That is one
moment on the world clock, such as the second an order was paid. The column has to carry its own
time zone. `TIMESTAMPTZ` in PostgreSQL and `datetimeoffset` in SQL Server are the two column types
that do.

The result is `null` when the column has no value in this row, so the type you get back is
`?Core\Time\Instant`. Test it for `null` before you use it.

The method throws a `LogicError` when the row has no column with that name, and when the column
carries no time zone of its own. SQLite stores no time zone with a value, so no column of a SQLite
database can be read this way. A column that holds a calendar day is read with `Core\Db\Row::date`,
and a clock reading with `Core\Db\Row::time`.
