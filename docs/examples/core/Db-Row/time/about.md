Reads one column of a row as a clock reading.

`Core\Db\Row::time` takes the name of a column and returns a `Core\Time\TimeOfDay`. That is an hour,
a minute and a second, such as the hour a shop opens or a train leaves. It has no day and no time
zone behind it, because a `TIME` column has neither. The same reading stands for every day.

The result is `null` when the column has no value in this row, so the type you get back is
`?Core\Time\TimeOfDay`. Test it for `null` before you use it.

The method throws a `LogicError` when the row has no column with that name, and when the column is
not a `TIME`. A column that holds a calendar day is read with `Core\Db\Row::date`, and one that
holds a day and a time together is read with `Core\Db\Row::instant`.

**The examples below** print the hour a shop opens, read a column that may have no value, and pick
out the shops that are open at nine.
