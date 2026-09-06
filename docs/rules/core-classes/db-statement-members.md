Five members run a statement. `query` and `queryAs<T>` answer a buffered `Db\Rows`, `execute` answers
a `Db\Write` carrying `affected`, `changed` and `lastId`, `executeMany` answers a count, and
`stream`/`streamAs<T>` answer an `Iterable` (`rule:core-classes/db-streaming`).

Buffering is the default because memory ranks last, and because the alternative breaks the commonest
loop in web programming — reading rows and writing per row — on a connection-busy rule that only
surfaces at run time. The rule is uniform across drivers even though SQL Server's MARS could lift it,
so that code written against one driver runs on all five.

`executeMany` does **not** open a transaction of its own; a caller who wants all-or-nothing writes
one, which composes with savepoint nesting and with retry. It is one prepare and N executions on
every driver, including MariaDB: its bulk-execute command ends on a refusal where the loop attempts
every later set, and it cannot route a statement that answers with a result set, so taking it would
mean one program leaving different rows in two servers. The price is N round trips where one command
would do, and it is recorded here rather than hidden.
