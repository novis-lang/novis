Runs one statement that reads rows, and returns every row as an object of the class you name.

`Core\Db\Connection::queryAs` runs the statement exactly as `Core\Db\Connection::query` does. Only
the rows are different. You write the class in angle brackets, as in `queryAs<Person>`, and each row
is built into a `Person`. The result is a `Core\Db\Rows` object, so `foreach`, `all()`, `count()` and
`first()` work as they do for `query`.

A class with the `#[Db\Derive]` attribute gets one property for each column with the same name. A
class with a `fromRow` method builds the object itself, so it can read several columns into one
property.

A text column fills only a `tainted string` property, and a binary column only a `tainted bytes`
property. A value in a database may come from a visitor. `tainted` keeps it out of a query or a page
until you check it. A plain `string` property does not compile.

If a column's value does not fit its property, `queryAs` throws an error that names the column.

The examples read rows into a class, build a class from a row with `fromRow`, and pass typed rows
from one method to the rest of a program.
