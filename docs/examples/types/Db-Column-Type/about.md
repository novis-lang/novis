Says what a result column was declared as in the database.

`Core\Db\Column::Type` is declared in the class `Core\Db\Column`. The method `type` returns it.

Every query result describes its columns, and this enum is the type part of that description. It
describes the column, not the values. A column declared as JSON and a column declared as text both
give text values, and this type tells them apart.

There is one case for each kind of column that Novis has its own type for. These are whole numbers,
exact decimals, text, raw bytes, booleans, the four date and time types, UUIDs and JSON. `Other` is
for everything else, including every array column. A program that shows, exports or pages through
rows reads this type to decide how to handle each column. It does not need to guess from the values.

**Good to know:** the cases are not ordered, so one case is not greater than another. A database
with no JSON type of its own reports its JSON columns as text.
