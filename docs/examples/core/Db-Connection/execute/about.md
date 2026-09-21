Runs one statement that changes the database, and says what it did.

`Core\Db\Connection::execute` is the method for an `insert`, an `update`, a `delete` or a
`create table`. It returns a `Core\Db\Write`. That object says how many rows changed, and it gives
the id the database made for a new row. A statement that reads rows belongs to
`Core\Db\Connection::query` instead.

A value never goes into the text of the statement. Write a `?` for each value, or a `:name` for
each, and pass the values as the second argument. The database keeps the statement and the values
apart, so a value that looks like SQL stays a value.

The examples write one row, change several rows with one statement, and name the values of a long
statement.
