Gives the first column of the first row a query answered.

`Core\Db\Rows::value` reads a single value out of a query that answers one. A `select count(*)` is the
everyday case: the result has one row with one column, and this method gives you that one value.

The type of the result is the column's own, so the method returns `mixed`. Write `as ?int` or
`as ?string` after the call to turn it into the type your program needs.

The result is `null` when the query matched no rows, and `null` as well when the first column of the
first row has no value. The two are not told apart. `Core\Db\Rows::count` is what separates them: a
result with no rows counts 0, and a result with one row counts 1.

**The examples below** read the number a count query answered, show the two times the result is
`null`, and build a summary line out of three one-value queries.
