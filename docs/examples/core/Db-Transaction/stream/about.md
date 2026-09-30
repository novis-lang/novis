Runs one statement that reads rows inside a transaction, and gives you the rows one at a time.

Use `Core\Db\Transaction::stream` for a result that is too large for memory, inside the function
you gave to `Core\Db\Connection::transaction`. It returns a `Core\Db\Stream`, and a `foreach` over
it reads one row from the database in each round of the loop. Only the current row is in memory.
The result includes the rows that the transaction has already written.

The transaction is busy until the last row is read. While rows remain, a second statement through
the transaction throws a `LogicError`, and the transaction cannot end. So read all the rows first,
and write after the loop.

**Good to know:** a loop that you leave early with `break` keeps the connection busy until the
request ends. To read only the first rows, put a `limit` in the statement. For a result that fits
in memory, use `Core\Db\Transaction::query`. With `query` you can write inside the loop.

**The examples below** read the rows that the transaction wrote one at a time, collect rows before
they write them, and add up a large table into a summary row.
