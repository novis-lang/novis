Runs one statement that reads rows, and gives you the rows one at a time.

Use `Core\Db\Connection::stream` for a result that is too large for memory. It returns a
`Core\Db\Stream`, and a `foreach` over it reads one row from the database in each round of the
loop. Only the current row is in memory, so ten million rows need about as much memory as ten.

The connection is busy until the last row is read. A second statement on the same connection
before that throws a `LogicError`. Read all the rows first and then run the next statement, or open
a second connection.

**Good to know:** a loop that you leave early with `break` keeps the connection busy until the
request ends. To read only the first rows, put a `limit` in the statement. For a result that fits
in memory, use `Core\Db\Connection::query`. It reads every row before it returns.

**The examples below** read a large table one row at a time, take only the first rows, and
calculate totals over a large table.
