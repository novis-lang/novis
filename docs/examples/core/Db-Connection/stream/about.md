Runs one statement that reads rows, and gives them to you one at a time.

`Core\Db\Connection::stream` is the method for a result that is too large to hold in memory. It
returns a `Core\Db\Stream` object, and a `foreach` over that object reads one row from the database
each time around the loop. Only the current row is in memory, so a table of ten million rows costs
about as much as a table of ten.

The connection is held for the whole walk. A second statement on the same connection while rows
remain is a `LogicError`. Read the rows, then run the next statement, or open a second connection
for it. The walk ends when the last row is read, and the connection is free again at that moment.
A loop you leave early with `break` keeps the connection busy until the request ends, so write the
bound into the statement with `limit` instead.

`Core\Db\Connection::query` is the sibling for a result that fits in memory. It reads every row
before it returns, so the connection is free right away.

The examples read a large table one row at a time, take only the first rows, and total a table
without holding it.
