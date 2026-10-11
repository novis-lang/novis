`$db->transaction($fn, {isolation?, readOnly?, retries?})` is the only transaction spelling. The
callable form is forced: with no destructors, an object-scoped transaction has no point at which to
roll back. A normal return commits, a throw rolls back and propagates, and a failed commit throws
`DbError`.

`Core\Db\Queryable` declares `query`, `queryAs`, `execute`, `executeMany`, `stream` and
`transaction`. `Connection` implements it and `Transaction implements Queryable by $connection`, so
the query surface is declared once and forwarded while the type system still expresses "this function
must run inside a transaction" and "this one does not care".

Two hazards a passable `Transaction` opens are closed explicitly. Using one after its `transaction()`
call returned throws. `$tx->rollBack($reason)` sets a rollback-only **flag** *and* throws
`Core\Db\RolledBack`, so an intervening `catch (Throwable)` cannot leave the transaction committed —
the owning frame acts on the flag, not on catching the signal.

Nesting on one connection issues `SAVEPOINT` and `ROLLBACK TO SAVEPOINT`, which removes the reason
`commit()`, `rollBack()` on the connection and `inTransaction()` each existed. `{retries: n}` re-runs
the callable on deadlock and serialization failure only, outermost transactions only, and defaults to
`0` because re-running a callable that sends mail is worse than surfacing the conflict. A retry
waits with exponential backoff and jitter by suspending the task, never by blocking the core.
`Isolation` is `ReadUncommitted`, `ReadCommitted`, `RepeatableRead`, `Snapshot` and `Serializable`,
and a level the driver lacks throws. `Transaction` is the first `Core` type to use `by` delegation.

DDL inside `transaction()` is already committed on MySQL and MariaDB, so a later throw rolls back
nothing there; PostgreSQL is transactional for DDL. No API shape prevents this.

**Declined:**
- **A `Db\Transaction` with its own query members.** It is the obvious way to type "inside a
  transaction", and it would be a second copy of the query surface
  (`rule:core-api/one-paradigm-per-operation`); delegation gives the same type safety with one
  implementation.
- **An explicit savepoint API, `commit()`, `rollBack()` on the connection, and `inTransaction()`.**
  Savepoint nesting removes the reason each existed.
- **Retrying by default.** Re-running a callable with side effects outside the database is worse than
  surfacing a conflict.
