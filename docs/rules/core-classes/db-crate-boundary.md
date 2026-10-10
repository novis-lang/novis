`crates/nvs-db` is one workspace crate below `nvs-stdlib`, depending on the runtime, the host's
stream and TLS, and configuration. `nvs-stdlib` depends on it and never the reverse: `Core\Db`'s
registry rows and reference cards stay where every other class's are, and the wire lives here, so the
graph stays a tree. A crate rather than a module, because five protocol implementations would
otherwise be rebuilt by every session that only touched `Core\Str`, and because the dependency set an
audit most wants to look at is worth having behind one manifest.

**No per-driver feature flags.** Every driver is in every binary, because `Core` means always
present and because the same binary should behave the same way everywhere — a feature matrix makes
"does this deployment speak MariaDB" a property of how someone built it. The cost is compile time and
binary size, spent to buy simplicity.

The rule the driver table encodes: **a codec is borrowed, a state machine is written.** Message
framing, value encoding and authentication mechanisms are large, fiddly and identical for everyone,
which is exactly what a sans-IO crate is. Sequencing — what to send next, what a park in the middle
means, when the connection is reusable — is where this project's own decisions live, and borrowing it
is what would have dragged an async runtime in. TDS has no such crate and is written by hand,
including its authentication and its tunnelled TLS: that is the price of SQL Server being in the
matrix at all.

| Driver | Borrowed codec | Written here |
|---|---|---|
| PostgreSQL | `postgres-protocol`, `postgres-types` | startup, SASL, the extended-query state machine, the reset |
| MySQL | `mysql_common` | handshake and auth sequencing, `COM_STMT_*` sequencing, the `LOCAL INFILE` refusal |
| MariaDB | `mysql_common`, with its own auth plugins and error table | the same, plus `RETURNING` |
| SQL Server | none | all of TDS 7.4 |
| SQLite | `rusqlite` | nothing on a wire |

SQLite has no wire: it is a C library doing synchronous file I/O, and it cannot park, so each call
goes to the host's blocking pool. A `Core` member either parks on readiness or hands the core to that
pool, and there is no third option. `rusqlite` brings the tree's one accepted C dependency,
`libsqlite3-sys`, recorded in the attribution ledger whose gate fails on any other C crate in the
graph; MariaDB's extra auth plugins are a Rust implementation or a documented refusal, never a second
C dependency.

**Memory:** per open connection, a read buffer, a write buffer, the statement cache sized by
`statement_cache`, and for a TLS connection the session state. That is O(in-flight connections),
released when the connection is destroyed and bounded by the pool's `max` per core. Per process it
adds nothing: the trust anchors are the same one set every TLS client in the tree uses.

**Declined:**
- **`sqlx`, `tokio-postgres`, `tiberius` or the sync `postgres` crate, with `tokio` behind a feature
  flag.** Each needs an async runtime that spawns, which would be a second reactor on a runtime whose
  design is that a core is never blocked and a task never migrates. The flag makes it worse, by
  turning the runtime's presence into a build property.
- **One crate per driver.** Five crates that all depend on the same shared half, are never used
  independently, and bring back the feature matrix this rule refuses. The audit argument for one
  separate crate does not extend to splitting it five ways.
