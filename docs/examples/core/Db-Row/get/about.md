Reads one column of a row without saying what type it is.

`Core\Db\Row::get` takes the name of a column and returns its value. The type of that value is the
column's own: an `INT` column gives an `int`, and a `TEXT` column gives a `string`. Your program
learns which one only while it runs, so the type of the result is `mixed`.

Write `as` after the call to turn the result into the type you need. `$row->get('seats') as ?int`
gives you a whole number or `null`.

The result is `null` when the column has no value in this row. The method throws a `LogicError` when
the row has no column with that name.

Where you know the type, a typed reader such as `Core\Db\Row::int` is the better choice, because it
writes the type down and checks it. Use `get` in a program that works on tables it was not written
for, such as a tool that copies rows.

**The examples below** read a column without naming its type, read a column that has no value, and
copy rows into another table.
