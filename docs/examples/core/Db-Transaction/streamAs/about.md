Runs one statement inside a transaction, and gives you each row as an object of the class you name,
one at a time.

`Core\Db\Transaction::streamAs` is `Core\Db\Transaction::stream` with the rows built into a class. You
write the class in angle brackets, as in `streamAs<Note>`, and each step of a `foreach` reads one row
from the database and builds one `Note` from it. Only the current object is in memory, so a table of
ten million rows costs about as much as a table of ten. Every row the transaction has written is
already there, and nobody else sees those rows until the transaction ends.

The class says how a row is read, exactly as it does for `Core\Db\Transaction::queryAs`. A class
marked `#[Db\Derive]` takes one property per column of the same name. A class that declares a
`fromRow` method builds the object itself, so it can read several columns into one property.

The walk holds the connection, and that is the connection the transaction has to end on. While rows
remain, a second statement through the transaction is a `LogicError`, and the end of the transaction
is refused as well. So read every row, keep what you need, then write. A loop you leave early with
`break` keeps the connection busy until the request ends, so write the bound into the statement with
`limit` instead.

The examples read the transaction's own rows as objects, build each object from several columns, and
write a reminder for every overdue invoice in one pass over a large table.
