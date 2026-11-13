The error a database throws when it does not accept a statement.

A key that is already taken, an empty required column and a deadlock all throw this one error. It
works the same with every database. Its `kind` property names the reason from one fixed list, so the
same code handles PostgreSQL, MySQL and SQLite. Your program can then act on the reason. A duplicate
email address needs a message for the user, and a deadlock can be tried again. The `sql` property is
the statement your program wrote.

**The server's own text:** `message` never contains text from the database server, because that
text can include a value your program sent. The server's text is in `serverMessage`. It is `null`
when the statement may have bound a `secret` value. `serverMessage` and `constraint` are a
`tainted string`. Tainted means the text came from outside your program, so check or escape it
before you use it.

**Good to know:** a mistake in the call itself is a `LogicError`. An example is a value the driver
cannot send.
