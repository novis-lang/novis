Checks that a name is a plain SQL identifier, such as a table name or a column name, and returns it
unchanged.

A value is sent to the database as a placeholder, but a table or column name cannot be. A name has to
be written into the text of the statement, and `Core\Db::quoteIdentifier` is the safe way to do that.
It accepts a name that starts with a letter or `_`, and continues with letters, digits or `_`. Every
other name throws a `LogicError`.

Text that arrived from a request is tainted, which means Novis does not allow you to write it into a
statement. The result of this method is not tainted, so you can use it.

**Good to know:** the result has no quotes or backticks around it. `Core\Db::quoteIdentifier` has no
connection, so it does not know which database the name is for, and the databases do not agree on
what a quote character means.
