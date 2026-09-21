Reads one column of a row as a calendar day.

`Core\Db\Row::date` takes the name of a column and returns a `Core\Time\Date`. That is a day in the
calendar, such as a birthday or the day an invoice is due. It carries no time of day and no time zone,
because a `DATE` column carries neither.

The result is `null` when the column has no value in this row, so the type you get back is
`?Core\Time\Date`. Test it for `null` before you use it.

The method throws a `LogicError` when the row has no column with that name, and when the column is
not a `DATE`. A column that also holds a time of day is read with `Core\Db\Row::instant`, and a clock
reading with `Core\Db\Row::time`.

**The examples below** print the day a task is due, read a column that may have no value, and pick out
the tasks whose day has passed.
