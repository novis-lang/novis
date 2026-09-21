Runs one statement once for every set of values, and says how many rows the whole batch wrote.

`Core\Db\Connection::executeMany` is the method for writing a group of rows with a single call. The
first argument is the statement, written once. The second is an array of arrays: one inner array per
run, each holding that run's values in the form `Core\Db\Connection::execute` takes. The result is a
whole number, the sum of what every run changed. An empty second argument writes nothing and the
result is `0`.

Every set must bind the same number of values, because there is only one statement. Two sets that
need a statement of a different length are two calls. There is no id for a new row here, and no rows
come back: a batch has no single run for either to belong to.

A batch is not a transaction. Each run stands on its own, so a failure in the middle leaves the
earlier writes in place. Wrap the call in `Core\Db\Connection::transaction` when you need all of it
or none of it.

The examples write three rows, give each row its own new value, and import a list of records.
