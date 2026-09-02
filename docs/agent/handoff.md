# Handoff

## State

**MySQL has met a real server.** `crates/nvs-db/tests/handshake.rs` opens a `MySqlConn` against
`tests/db/compose.yaml`'s `mysql` service and asserts what only that server can say: § 3's upgrade as
the `Ssl_version` *session* status variable, `CURRENT_USER()`, and a wrong password refused as a
`ServerError` carrying `28000`/`1045`; § 7 nesting to a `SAVEPOINT` and rolling back to it, where the
surviving rows are `1,3` and a driver that had sent a plain `ROLLBACK` answers `3`; and § 8's
duplicate key as `UniqueViolation`/`1062`/`23000`. The MySQL helpers are
`crates/nvs-db/tests/handshake.rs:176-270` (`mysql`, `mysql_connect_as`, `mysql_open`,
`mysql_one_value`, `mysql_run`, `mysql_try`). PostgreSQL's keep the unprefixed names deliberately:
`docs/agent/loop-goal.toml:2764` names its two test functions, so renaming them breaks an acceptance
check.

**The `mysql` service serves the `certs` volume's leaf now** (`tests/db/compose.yaml:98`), and
`tools/db-matrix.py`'s anchor for it is `/certs/ca.crt` rather than the data directory's `ca.pem`.
The playbook bullet owns why. `python tools/db-matrix.py --driver postgres --driver mysql --no-up` is
`2/2 drivers ok`, and the mysql leg now connects rather than skipping every case in the crate.

**MariaDB is a struct with no driver.** `crates/nvs-db/src/conn.rs:707` is `MariaConn` with no
`connect`, and the three cases stage 6 names for it exist nowhere. With MySQL's leg landed that is the
largest hole in this goal, and it is a driver rather than a slice.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for a
shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own
ADR, not a slice. Untouched this session.

**`orient.py` gaps: none left to report — the fix landed.** `[context] adrs` now carries § 3, § 7 and
§ 8 beside the standing three, and its comment records that the list is the only selector.

## Next group

**More of that same server, one file set: `crates/nvs-db/tests/handshake.rs` and
`crates/nvs-db/tests/pool_reuse.rs`. Every helper the three slices need is already in the first file;
the second is PostgreSQL-only and is twinned the same way.**

- [ ] **The parking twin for MySQL** — a read this connection is waiting on parks the coroutine, so
      another task takes a turn. `SELECT SLEEP(0.5)` is what `pg_sleep(0.5)` is, and the case is the
      PostgreSQL one with `mysql_open`/`mysql_one_value` substituted. ADR 0067 § 3.
      `crates/nvs-db/tests/handshake.rs:337`, `crates/nvs-db/tests/handshake.rs:81`.
- [ ] **§ 13's reset against that server** — after `MySqlConn::reset` no temporary table, no session
      variable and no cached statement survives, which is the *property* § 13 states rather than the
      `COM_RESET_CONNECTION` the peer cases already hold. ADR 0067 § 13.
      `crates/nvs-db/src/mysql.rs:1453`, `crates/nvs-db/tests/handshake.rs:464`.
- [ ] **`pool_reuse.rs`'s identity assertion on MySQL** — `CONNECTION_ID()` is what `pg_backend_pid()`
      is, so two requests on one core share one connection there too, and a release past the bounds
      does not. ADR 0067 § 13. `crates/nvs-db/tests/pool_reuse.rs:62`,
      `crates/nvs-db/tests/pool_reuse.rs:128`.

## Backlog

- MariaDB's driver: `MariaConn` has no `connect`, `crates/nvs-db/src/conn.rs:707`.
- MariaDB serves no certificate a client can verify, and SQL Server keeps its own inside the instance
  — `tests/db/compose.yaml:134`, and the `certs` volume is the answer for the first.
- `Core\Db::open`'s shape parameter waits on a registry type — `nvs_stdlib::db` known gap 1, which is
  what stage 2's acceptance check is open on.
- MySQL's § 9 type map has never been read off a real server's rows — `crates/nvs-db/src/mysql.rs:2579`.
