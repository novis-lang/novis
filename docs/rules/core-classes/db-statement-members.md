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
would do, and it is recorded here rather than hidden. An empty set list is a no-op answering `0`.
`INSERT … ON DUPLICATE KEY UPDATE` reports `2` for a row it updated on MySQL and MariaDB; that is the
server's number and it is not rewritten.

- **Memory:** a buffered `query` holds its whole result for the rest of the request, counted against
  the request's memory cap. A program that does not reach for `stream` meets the cap, not a slow path.

**Declined:**
- **A transaction inside `executeMany`.** A hidden `BEGIN` in a member whose name does not mention
  one would silently change lock duration on a large batch, and the caller's own `transaction()`
  already composes (`rule:core-classes/db-transactions`).
- **MariaDB's `COM_STMT_BULK_EXECUTE`.** Taking it would spend a property of the surface on round
  trips. `MARIADB_CLIENT_BULK_UNIT_RESULTS`, which answers per set, reopens it only once a real server
  is shown to continue past a refused set under it.
