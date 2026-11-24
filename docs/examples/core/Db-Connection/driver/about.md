Says which database system a connection speaks to.

`Core\Db\Connection::driver` returns a `Core\Db\Driver` case: `Sqlite`, `Postgres`, `MySql`,
`MariaDb` or `SqlServer`. A case is a value, so your program compares it with `==` and never reads
a name out of a string.

The value comes from the connection itself, not from the settings file. A connection that
`Core\Db::connect` opened from a `[db.<name>]` block and one that `Core\Db::open` opened from a
settings value both return it the same way.

Programs call this when a statement does not work on every database, and when a program supports
some databases and not others. A closed connection throws a `LogicError` here.
