A streaming statement's read state is split from the borrow and parked on the connection, on every
driver, so a walk advanced by a later call holds one row rather than a result set.

What is parked is what the result set described, what will end it, and the trace event the statement
is timed by — `crates/nvs-db/src/pg.rs`'s `PgCursor` is the shape, and each driver's is the same kind
of value: the column definitions and the wire's sequence position on MySQL and MariaDB, `COLMETADATA`
on SQL Server. The buffered members keep the same state beside a borrow of the connection, so one row
reader serves both paths per driver and they cannot disagree about what ends a stream.
`State::Streaming` is what refuses the second statement (`rule:core-classes/db-connection-busy-state`),
read off a cell rather than off a lifetime.

**SQLite streams on one pinned thread per open walk.** `rusqlite`'s rows borrow the statement, which
borrows the connection, so there is no value to park; the statement is stepped on a thread from the
blocking pool and each row handed across instead. The thread is released when the walk is drained,
dropped, or its task ends — O(open streams), never O(requests served).

**Buffering is refused on every driver, in every disguise**, because it breaks the member's one
promise and makes a request's memory a function of a table's size. A driver whose protocol cannot be
advanced between calls throws a `RuntimeError` naming the driver and naming `query`; that refusal is
the fallback for a protocol that has no parked form, not a state any of the five drivers is in.

**An abandoned stream drains rather than poisons.** Each wire protocol frames its remaining rows
self-describingly to a terminating packet or `DONE` token, so the read back to a message boundary is
deterministic, bounded by the result set the caller asked for, and the connection returns to `Idle`
and to the pool. A read that *fails* mid-message is `Poisoned` and closed, as it is for every other
statement.
