A `deadline` bounds when cancellation is **requested**, not when the call returns.

A child blocked mid-statement cannot simply be abandoned: a database connection with an unread
result set is unusable, so the statement is cancelled through the driver's own mechanism (`KILL
QUERY`, `PQcancel`, `sqlite3_interrupt`) and the
call waits until that connection is back in a known state. That drain is bounded in turn by the
connection's own `timeout`, after which the connection is **closed rather than returned to the
pool**.

So `deadline: 2s` can return at two seconds plus one connection timeout in the pathological case.
Overrunning a stated deadline by a bounded amount is the correct trade against handing a poisoned
connection back to a pool, where it would fail some later, unrelated request. A deadline is a bound
on when the work stops being asked for; it is not a hard wall-clock guarantee on return.

**Not on disk.** A deadline cancels every child still running, and the call waits for them
(`crates/nvs-host/src/group.rs:60`). `nvs_db::Connection::settle` sends PostgreSQL's
`CancelRequest` over a second TLS connection and drains the wire back to `Idle` before an instant
(`crates/nvs-db/src/pg.rs`). Nothing calls it yet: an interrupted statement still closes its
connection, no `KILL QUERY`, `sqlite3_interrupt` or TDS attention is sent, and the drain is not
bounded by the connection's `timeout`.

**Declined:**
- **Returning at the deadline and abandoning the blocked child.** It would make the deadline exact,
  and it breaks `rule:concurrency/nothing-is-still-running-when-a-call-returns` and puts a
  connection with an unread result set back in the pool.
