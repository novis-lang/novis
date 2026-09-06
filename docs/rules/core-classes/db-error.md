`Core\Db\DbError extends RuntimeError` and is readonly: `kind`, `sqlState`, `driverCode`,
`constraint`, `sql`. `ErrorKind` normalises the conditions applications actually branch on —
`UniqueViolation`, `ForeignKeyViolation`, `NotNullViolation`, `CheckViolation`, `Deadlock`,
`SerializationFailure`, `ConnectionLost`, `Timeout`, `Syntax`, `Permission`, `Other` — across four
drivers and five dialects, MariaDB carrying its own code table rather than MySQL's. SQLite's
`SQLITE_BUSY` and `SQLITE_LOCKED` map to `Deadlock`, so `rule:core-classes/db-transactions`'s retry
option works there too. The raw values stay available for what normalisation does not cover.

A class per condition was refused: it would add ten types to a deliberately small closed exception
set, and some boundaries are driver-dependent.

**Bound parameters never appear on the error, in the message, or in a trace** — a `Throwable` message
is a `secret` sink. The SQL text may, being developer-authored. The four raw values ride the throw on
one boxed slice, which is eight bytes narrower than the single slot it replaced and costs one
allocation on a path that is already allocating the object, the message and the backtrace.
