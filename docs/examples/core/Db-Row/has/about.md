Tests whether the row has a column with this name.

`Core\Db\Row::has` takes the name of a column and returns `true` or `false`. It returns `true` for
every column the row carries. A column with no value in this row is still a column, so the answer
for it is `true` as well. It returns `false` for a name that belongs to no column of this row.

This method never throws an error, and that is what it is for. The typed readers do throw: both
`Core\Db\Row::int` and `Core\Db\Row::string` throw a `LogicError` when the row has no column with
that name. Test the name with `has` first, and you read it without catching an error. That matters
when the name came in with a request, because then your program did not choose it.

**The examples below** test a column before reading it, read rows from two tables with one function,
and read only the columns a request asked for.
