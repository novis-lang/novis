`Core\Db\DbError extends RuntimeError` and is readonly: `kind`, `sqlState`, `driverCode`,
`constraint`, `sql`. `ErrorKind` normalises the conditions applications actually branch on —
`UniqueViolation`, `ForeignKeyViolation`, `NotNullViolation`, `CheckViolation`, `Deadlock`,
`SerializationFailure`, `ConnectionLost`, `Timeout`, `Syntax`, `Permission`, `Other` — across four
drivers and five dialects, MariaDB carrying its own code table rather than MySQL's. SQLite's
`SQLITE_BUSY` and `SQLITE_LOCKED` map to `Deadlock`, so `rule:core-classes/db-transactions`'s retry
option works there too. The raw values stay available for what normalisation does not cover. A
driver that exposes only SQLSTATE and a vendor integer is why code elsewhere matches on
`"Duplicate entry"` or hard-codes `1062`; a normalised kind removes that string match.

**Bound parameters never appear on the error, in the message, or in a trace** — a `Throwable` message
is a `secret` sink. The SQL text may, being developer-authored. The four raw values ride the throw on
one boxed slice, which is eight bytes narrower than the single slot it replaced and costs one
allocation on a path that is already allocating the object, the message and the backtrace. All four
are nullable, so a slot never written already reads `null` and absence costs nothing.

**Declined:**
- **A class per condition.** It would add ten types to a deliberately small closed exception set, and
  some boundaries are driver-dependent.
- **Carrying the raw values inline on the fault.** The fault is the error half of every helper's
  result, so its width is paid on every successful call; five pairs inline would nearly triple it, and
  keeping the single pair beside a boxed slice saves nothing and adds a third arm at every site that
  destructures a fault.
