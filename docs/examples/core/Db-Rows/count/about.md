Says how many rows a query returned.

`Core\Db\Rows::count` returns the number of rows the statement returned, as a whole number. The
number is exact. Novis reads every row before `query` returns, so this is the length of a result you
already have. It is never an estimate.

A query that matched nothing gives 0. Counting does not use the rows up. You can count a result, read
it, and count it again, and the number is the same every time.

This counts the rows a `select` returned. For the number of rows an `insert`, an `update` or a
`delete` changed, read `Core\Db\Write::affected` instead.

**The examples below** show how many rows a search found, and do something else when nothing matched.
The last one works out how many pages a list of results needs.
