Runs one statement that reads rows, and returns every row as an object of the class you name.

`Core\Db\Connection::queryAs` runs the same statement `Core\Db\Connection::query` runs, with the
same values and the same bounds. The one difference is what a row becomes. You write the class in
angle brackets, as in `queryAs<Person>`, and each row is built into a `Person`. The result is a
`Core\Db\Rows` object, so `foreach`, `all()`, `count()` and `first()` all work as they do for
`query`.

There are two ways to say how a row fills the class. A class marked `#[Db\Derive]` takes one
property per column of the same name, and the code that fills it is written for you. A class that
declares a `fromRow` method builds the object itself, so it can read several columns into one
property.

A column whose value does not fit the property it belongs to is an error, and the error names the
column.

The examples read rows into a class, build a class from a row with `fromRow`, and pass typed rows
from one method to the rest of a program.
