Tells you the row count the server itself reported, or `null` where it reported none.

`Core\Db\Connection::execute` returns a `Core\Db\Write`, and `changed` is one of the counts on it.
Its type is `?uint`: a whole number, or `null`. Where it is a number, it is the same number
`Core\Db\Write::affected` gives.

`null` means the statement has no row count at all. A `create table` is such a statement on
PostgreSQL. `affected` turns that into `0`, so `changed` is the only place where "the server gave no
count" and "the statement changed no rows" still look different.

Not every server can tell those two apart. SQLite reports a count for every statement it runs, so on
SQLite `changed` is never `null`.

Where your program needs a number to work with, test the count for `null` first.

The examples read the count a server reported, tell no count apart from no rows, and add up the rows
a clean-up removed.
