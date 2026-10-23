Runs one statement inside a transaction, and returns every row as an object of the class you name.

`Core\Db\Transaction::queryAs` runs the statement exactly as `Core\Db\Transaction::query` does. Only
the rows are different. You write the class in angle brackets, as in `queryAs<Note>`, and each row is
built into a `Note`. The result is a `Core\Db\Rows` object, so `foreach`, `all()`, `count()` and
`first()` work as they do for `query`.

A class with the `#[Db\Derive]` attribute gets one property for each column with the same name. A
class with a `fromRow` method builds the object itself, so it can read several columns into one
property.

A text column fills only a `tainted string` property, and a binary column only a `tainted bytes`
property, as for `Core\Db\Connection::queryAs`.

The objects can contain rows the transaction wrote. If the transaction is rolled back, those rows no
longer exist in the database. The objects you already built still exist.

The examples read what the transaction wrote as objects, build each object from several columns, and
claim a batch of jobs.
