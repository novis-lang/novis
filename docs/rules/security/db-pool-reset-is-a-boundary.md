A connection released at request teardown returns to a per-core pool, and **a released connection is
reset before it is reusable**. The reset is not best-effort: a connection that cannot be proven clean
is closed, and a driver with no reset primitive is not poolable at all.

What the reset must remove is stated as a **property, not a command list**: after it, no transaction,
no temporary table, no session variable, no assumed role, no advisory lock, no listener, no open
cursor and no prepared statement the cache does not still account for. A backend added later satisfies
that property or is not pooled.

**The pool key includes every credential**, so two configuration blocks are two pools and two database
users never share a connection. It is additionally scoped to the configuration generation it was read
from: a reload can publish the same block name under a different user, and a pool keyed on the name
alone would hand the new generation's request a connection authenticated as the old one's. A reload
is therefore two pools, and the superseded generation's connections retire against their own
`lifetime`.

The pool is **per core and never shared between cores**: a task never migrates, so the acquire path
takes no lock, which is the reason to prefer it over one process-wide pool. Per backend, PostgreSQL
rolls back, then runs `SET SESSION AUTHORIZATION DEFAULT`, `RESET ROLE`, `RESET ALL`, `CLOSE ALL`,
`UNLISTEN *` and `pg_advisory_unlock_all()` and drops the session's temporary schema. `RESET ALL`
skips the role and the session authorization, so each has its own command; MySQL, MariaDB and SQL Server use their atomic reset command, which
also drops prepared statements, so the statement cache goes with it; a SQLite connection has no
session state, and rolling back an open transaction is the whole reset. The pool exists to keep a
connection's handshake and statement cache across requests; Novis code cannot observe whether a
handshake happened.

Bounds are finite with nothing configured: `[db.<name>.pool]` writes `max` (per core, default 16),
`idle`, `lifetime` (a connection is retired regardless of health) and `acquire` (waiting for a free
connection throws rather than hangs). `max` being per core makes a deployment's ceiling on the server
`cores × max`, the number an operator sizes `max_connections` against. An `open` whose settings hash
matches a block's takes that block's bounds, since it is already the same pool; any other settings
literal gets the defaults. `pool = false` restores connect-per-request exactly, for an audited
environment where each connection must map to one request, and may be written unscoped under `[db]`
to reach every connection the process opens, including one a program's own settings named. Only the
switch is unscoped: a `[db.pool]` table of bounds is refused at boot, because no one server's
`max_connections` is what a pool keyed on a credential hash would be sized against.

- **Memory:** each core holds up to `max` open connections per pool key, with their statement caches,
  for as long as `lifetime` allows. That is per process and bounded by configuration, never by
  traffic served.

**Declined:**
- **`DISCARD ALL` on PostgreSQL.** It also deallocates every prepared statement, which would throw
  away the statement cache (`rule:core-classes/db-one-api`) that pooling exists to preserve.
- **One process-wide pool.** It needs a lock on every acquire.
