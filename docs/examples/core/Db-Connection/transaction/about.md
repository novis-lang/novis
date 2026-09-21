Runs your work inside a database transaction.

`Core\Db\Connection::transaction` takes a function and runs it. Your function is given a
`Core\Db\Transaction`, which has the same `query`, `execute` and `executeMany` methods a connection
has. When the function returns, the work is committed, and `transaction` returns what the function
returned. When the function throws an error, the work is rolled back and the error travels on.

There is no method that begins, commits or rolls back a transaction on its own, so a transaction
cannot be left open by an early return.

Inside the function, `Core\Db\Transaction::rollBack` gives up. It takes a reason and throws
`Core\Db\RolledBack` with that reason.

A second `transaction` inside the first is a savepoint. Giving up on the inner one undoes only its
own work, so a program can skip one bad row and keep the rest.

You can also name an isolation level, ask for a read-only transaction, and set how many times a
deadlock may re-run the function.

The examples move money between two accounts, give up on an order that cannot be filled, and import
a list where one row is bad.
