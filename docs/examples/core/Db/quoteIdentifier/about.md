Checks that a name is a plain SQL identifier, such as a table or column name, and returns it
unchanged.

A value goes to the database as a placeholder, but a table or column name cannot. The name is written
into the text of the statement, so `Core\Db::quoteIdentifier` checks it first. It accepts a name that
starts with a letter or `_` and continues with letters, digits or `_`. Every other name throws a
`LogicError`. A few names, such as `user` and `current_timestamp`, also throw. The database reads
them as values, such as its own user name.

Text from a request is tainted: Novis does not allow you to write it into a statement. The result of
this method is not tainted, so you can use it.

The method checks the syntax only. A visitor who chooses the name can still pick any column, such as
`password_hash`, and sort by it. Check a name a visitor chooses against a list of allowed names, as
example 03 does.

**Good to know:** the result has no quotes or backticks around it. The method has no connection, so it
does not know which database the name is for.
