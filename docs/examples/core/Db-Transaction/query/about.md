Runs one statement that reads rows inside a transaction, and returns all of them.

`Core\Db\Transaction::query` is the method for a `select` inside the work you passed to
`Core\Db\Connection::transaction`. It returns a `Core\Db\Rows` object. A `foreach` over that object
reads one row at a time, `count()` gives the number of rows, and `first()` gives the first row or
`null`. The method is the same one the connection has, so a statement you already wrote does not
change when you move it inside a transaction.

What changes is what the read sees. Every row the transaction has written is already there, before
the transaction ends. Nobody else sees those rows until it does.

Every row is read into memory before the method returns, so you can write inside the loop that
reads them. A value never goes into the text of the statement. Write a `?` for each value, or a
`:name` for each, and pass the values as the second argument.

The examples read what the transaction just wrote, write while reading the rows, and let a read
decide whether the work is kept.
