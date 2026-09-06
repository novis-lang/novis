Where an operator wrote a local endpoint, the string they write is the one their deployment already
holds. For `driver = "postgres"` the `host` names the **directory** and the socket is derived —
`/var/run/postgresql` with `port = 5432` is `/var/run/postgresql/.s.PGSQL.5432` — because that is
what libpq, `psql`, PDO and every Postgres tool take. MySQL and MariaDB take the socket **file**,
because their socket has no naming convention to derive one from, and that too is the string those
deployments already hold.

**MSSQL refuses a path.** TDS has no `AF_UNIX` transport, so the driver reports it as a target it
does not speak rather than as a file it could not open. SQLite is untouched: its path *is* the
database.

The cost is one piece of protocol trivia per driver, encoded where that driver's trivia belongs.

**Not shipped.** No driver in `crates/nvs-db/` opens a Unix socket yet — `crates/nvs-db/src/pg.rs`
holds neither the derivation nor the transport, and `crates/nvs-db/src/tds/mod.rs` has no refusal to
report.
