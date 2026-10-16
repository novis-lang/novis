Tells you the key the database gave to the row a statement inserted.

`Core\Db\Connection::execute` returns a `Core\Db\Write`, and `lastId` is the key on it. The type is
`?uint`: a whole number, or `null` where the statement handed no key back.

This is the number a program needs straight after an insert, because the rows that belong to the new
one are written with it: the lines of an order, the tags of an article, the members of a group.

A statement that inserts several rows hands back one key, and it is the last row's. Where your
program needs every key, insert the rows one at a time.

The key belongs to the write that produced it. Statements that run later on the same connection do
not change what an earlier `Core\Db\Write` answers, so the key is still the right one after other
work has happened.

On PostgreSQL the key comes from a `returning` clause, because that server has no key of its own to
report.

The examples read the key of a new row, ask a statement that inserted nothing for one, and write the
lines that belong to an order.
