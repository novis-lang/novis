Runs one statement that reads rows inside a transaction, and gives them to you one at a time.

`Core\Db\Transaction::stream` is the method for a result that is too large to hold in memory, inside
the work you passed to `Core\Db\Connection::transaction`. It returns a `Core\Db\Stream` object, and a
`foreach` over that object reads one row from the database each time around the loop. Only the
current row is in memory, so a table of ten million rows costs about as much as a table of ten. Every
row the transaction has written is already there, and nobody else sees those rows until the
transaction ends.

The walk holds the connection, and that is the connection the transaction has to end on. While rows
remain, a second statement through the transaction is a `LogicError`, and the end of the transaction
is refused as well. So read every row, then run the next statement. A loop you leave early with
`break` keeps the connection busy until the request ends, so write the bound into the statement with
`limit` instead.

`Core\Db\Transaction::query` is the sibling for a result that fits in memory. It reads every row
before it returns, so you can write inside the loop that reads them.

The examples read the transaction's own rows one at a time, collect the rows before they write them,
and total a large table into a summary row.
