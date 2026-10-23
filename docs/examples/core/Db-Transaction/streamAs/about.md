Runs one statement inside a transaction and gives you each row as an object of your class, one row
at a time.

`Core\Db\Transaction::streamAs` works like `Core\Db\Transaction::stream`, and it builds an object
from each row. You write the class in angle brackets, as in `streamAs<Note>`. Each round of a
`foreach` reads one row and builds one `Note`. Only the current object is in memory.

The class says how a row is read, as for `Core\Db\Transaction::queryAs`. A class with the
`#[Db\Derive]` attribute gets one property for each column with the same name. A class with a
`fromRow` method builds the object itself. A text column fills only a `tainted string` property, and
a binary column only a `tainted bytes` property.

**Good to know:** the transaction is busy until the last row is read. While rows remain, a second
statement in the transaction throws a `LogicError`. Read all the rows, keep what you need, and write
after the loop. To read only the first rows, put a `limit` in the statement.

The examples read the rows that the transaction wrote as objects, build each object from several
columns, and write a reminder for every overdue invoice.
