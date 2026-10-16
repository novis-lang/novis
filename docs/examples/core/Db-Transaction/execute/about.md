Runs one statement inside a transaction, and says what it did.

`Core\Db\Transaction::execute` is the method for an `insert`, an `update`, a `delete` or a
`create table` inside the work you passed to `Core\Db\Connection::transaction`. It returns a
`Core\Db\Write`. That object says how many rows changed, and it gives the id the database made for
a new row. The method is the same one the connection has, so a statement you already wrote does not
change when you move it inside a transaction.

What changes is when the write is kept. Every write of one transaction is saved together, at the
moment your function returns. If your function throws an error, or calls
`Core\Db\Transaction::rollBack`, none of them is saved.

A value never goes into the text of the statement. Write a `?` for each value, or a `:name` for
each, and pass the values as the second argument.

The examples write one row and read what it did, give up on a write, and save an order together
with the lines that belong to it.
