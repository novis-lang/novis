Says how many rows a query answered.

`Core\Db\Rows::count` returns the number of rows the statement answered, as a whole number. The number
is exact. Novis reads every row before `query` returns, so this is the length of a result you already
hold, and never an estimate.

A query that matched nothing returns 0. Counting does not use the rows up: you can count a result,
read it, and count it again, and the answer is the same every time.

This counts the rows a `select` answered. For the number of rows an `insert`, an `update` or a
`delete` changed, read `Core\Db\Write::affected` instead.

**The examples below** say how many rows a search found, take another path when nothing matched, and
work out how many pages a list of results needs.
