Runs one statement once for every set of values inside a transaction, and says how many rows the
whole batch wrote.

`Core\Db\Transaction::executeMany` is the method for writing a group of rows with a single call. The
first argument is the statement, written once. The second is an array of arrays: one inner array per
run, each holding that run's values in the form `Core\Db\Transaction::execute` takes. The result is
a whole number, the sum of what every run changed. An empty second argument writes nothing and the
result is `0`.

Every set must bind the same number of values, because there is only one statement. Two sets that
need a statement of a different length are two calls.

The difference from `Core\Db\Connection::executeMany` is what happens when a run in the middle
fails. On the connection, the runs before it stay in the database. Here the transaction gives up, so
the whole batch is removed again, together with everything else the transaction wrote.

The examples write three rows, show a batch that is removed again, and import a list of records
under a row that describes the import.
