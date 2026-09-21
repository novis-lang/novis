Runs one statement inside a transaction, and returns every row as an object of the class you name.

`Core\Db\Transaction::queryAs` runs the same statement `Core\Db\Transaction::query` runs, with the
same values and the same bounds. The one difference is what a row becomes. You write the class in
angle brackets, as in `queryAs<Note>`, and each row is built into a `Note`. The result is a
`Core\Db\Rows` object, so `foreach`, `all()`, `count()` and `first()` all work as they do for
`query`.

There are two ways to say how a row fills the class. A class marked `#[Db\Derive]` takes one
property per column of the same name, and the code that fills it is written for you. A class that
declares a `fromRow` method builds the object itself, so it can read several columns into one
property.

The objects are built from the rows the transaction has written, before the transaction ends. If
the transaction gives up, those rows are gone, and the objects you already built are still there.

The examples read what the transaction wrote as objects, build each object from several columns,
and claim a batch of jobs.
