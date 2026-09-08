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

**MariaDB is the one still owing this.** `crates/nvs-db/src/mysql.rs` opens the socket file as
written and `crates/nvs-db/src/pg.rs` derives the engine's own name in its `socket_endpoint`, both
over the second arm of `crate::conn::Endpoint` — the address-or-path a driver's `connect` takes;
`crates/nvs-db/src/tds/mod.rs` refuses a path in `TdsTarget::resolve`, as `BlockError`'s
`NoSocketTransport`. `crates/nvs-db/src/maria.rs` still takes an address.
