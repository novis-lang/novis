Runs one statement inside a transaction and gives you each row as an object of your class, one row
at a time.

`Core\Db\Transaction::streamAs` works like `Core\Db\Transaction::stream`, and it builds an object
from each row. You write the class in angle brackets, as in `streamAs<Note>`. Each round of a
`foreach` reads one row from the database and builds one `Note`. Only the current object is in
memory.

The class decides how a row is read, the same way as for `Core\Db\Transaction::queryAs`. A class
with the `#[Db\Derive]` attribute gets one property for each column with the same name. A class
with a `fromRow` method builds the object itself, so it can read several columns into one property.

**Good to know:** the transaction is busy until the last row is read. While rows remain, a second
statement through the transaction throws a `LogicError`. Read all the rows and keep what you need,
and write after the loop. To read only the first rows, put a `limit` in the statement.

**The examples below** read the rows that the transaction wrote as objects, build each object from
several columns, and write a reminder for every overdue invoice.
