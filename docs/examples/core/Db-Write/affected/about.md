Tells you how many rows a statement changed.

`Core\Db\Connection::execute` returns a `Core\Db\Write`, and `affected` is the count on it. It is a
whole number and never `null`, so it needs no unwrapping: an `insert` of three rows answers `3`, and
an `update` that matched two rows answers `2`.

An `update` or a `delete` whose `where` matched no row answers `0`. This is the answer most programs
act on. It says that the row you meant to change is not there, or that somebody else changed it
first, which is what makes a conditional `update` a way to let exactly one worker take a job.

A statement that has no rows to count, such as `create table`, also answers `0`.
`Core\Db\Write::changed` is where those two are told apart.

The count belongs to the statement that answered it. Later statements on the same connection do not
change what an earlier `Core\Db\Write` says.

The examples count the rows an update changed, find out that nothing matched, and let one worker
take a job.
