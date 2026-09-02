# Handoff

## State

**Both servers the matrix could not verify now present a certificate a client can anchor, and both legs
run.** MariaDB takes `--ssl-ca`/`--ssl-cert`/`--ssl-key` at `/certs/…` exactly as the `mysql` service
does. SQL Server has neither a flag nor an environment variable for one, so its entrypoint is overridden
to write an `mssql.conf` `[network]` section and then `exec` the image's own `sqlservr`; the `certs` job
emits `mssql.key`, the same key under a second name owned by uid 10001, because one file cannot be 0600
for two uids. `tests/db/compose.yaml`'s two block comments own both pieces of reasoning. Verified here:
a MariaDB client completes TLS 1.3 verifying against `/certs/ca.crt`, SQL Server's errorlog names
`/certs/server.crt` as what it loaded, and `python tools/db-matrix.py --driver mariadb --driver mssql`
is `2/2 drivers ok`.

**Those two legs assert nothing yet.** `MariaConn` and `MssqlConn` have no `connect`, so every case in
`crates/nvs-db/tests/handshake.rs` returns early for them — the leg proves the fixture, not the driver.
Stage 6's comment in `docs/agent/loop-goal.toml` says so and no longer claims either server is
unverifiable; a leg reporting `n/a` again is now a fixture regression rather than the expected state.

**MySQL's real-server coverage is unchanged**: § 3, § 7, § 8 and § 13 across five cases in
`crates/nvs-db/tests/handshake.rs`, whose `mysql()` fixture helper at :187 is what a `mariadb()` mirrors,
plus `crates/nvs-db/tests/pool_reuse.rs` twinned three ways.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for a
shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own ADR,
not a slice. Untouched this session.

**`docs/agent/goals/5-database.toml` had drifted** from the live `loop-goal.toml` by the previous
session's three `[context] adrs` additions; it is byte-identical again, so the chain's next
`goal-switch.py` no longer reverts them.

**`orient.py` gaps: none.**

## Next group

**The MariaDB driver, one file set: `crates/nvs-db/src/conn.rs`, `crates/nvs-db/src/mysql.rs` and
`crates/nvs-db/tests/handshake.rs`. `MySqlConn` is the worked example for all three slices, and none of
them touches the compose file again.**

- [ ] **`MariaConn::connect`** — `mysql.rs`'s codec with MariaDB's own auth plugins and its own error
      table, over the same `NvsTls` parking stream. ADR 0067 § 3 for the handshake, § 8 for the code
      table that is not MySQL's. `crates/nvs-db/src/conn.rs:707`, `crates/nvs-db/src/mysql.rs:1281`,
      `crates/nvs-db/src/lib.rs:153`.
- [ ] **`the_mariadb_auth_plugins_are_implemented_in_rust_or_refused_by_name`** — ADR 0051 § 4 named
      this case in advance so the answer would come from a test rather than from convenience: a plugin
      is implemented here or refused by name, never shelled out to. `docs/agent/loop-goal.toml:2916`,
      `crates/nvs-db/tests/handshake.rs:424`.
- [ ] **`mariadb_returning_is_available_and_mysqls_is_not` and
      `execute_many_uses_the_bulk_protocol_on_mariadb`** — the two behaviours that make MariaDB its own
      driver rather than a MySQL flag, one test each, over a `mariadb()` fixture helper mirroring
      `mysql()`. `docs/agent/loop-goal.toml:2913`, `crates/nvs-db/tests/handshake.rs:187`.

## Backlog

- `Core\Db::open`'s shape-parameter registry type — `nvs_stdlib::db` known gap 1, wants an ADR.
- `pool = false` has no MySQL twin in `pool_reuse.rs`; the other three bounds do.
- `MssqlConn::connect` — TDS 7.4, the largest piece of new wire code; `crates/nvs-db/src/conn.rs:712`.
- The SQLite leg of the matrix — stage 6's `want` list.
- § 11's `slow_query` line against a real server — stage 9.
