# Handoff

## State

**Goal `m8-db-queue`, stage 4 is closed.** `Core\Db\Connection::stream` answers on all five drivers
and every proof it owed is on disk: `crates/nvs-stdlib/tests/db_stream.rs` gated on
`NVS_DB_MATRIX_DRIVER` the way `queue.rs` beside it is, the `.nvst` case over a `:memory:` block, and
`tools/db-matrix.py`'s `SUITES` running the suite on every leg. All five legs were run against the
containers this session — mysql, mariadb, postgres, mssql and sqlite each `ok`.

**The uniform busy rule was one driver short, and the `.nvst` case is what found it.** SQLite built
its refusal with `io::Error::other`, so a second statement on a streaming SQLite connection reached a
program as `Core\Db\DbError` where the other four answered `LogicError`. It is `InvalidInput` now
(`crates/nvs-db/src/sqlite.rs:1454`); the trap behind it is in the playbook.

Nothing is blocked. Stage 1 is the carried floor and the driver's to run.

## Next group

**Stage 5: `serverVersion`, from the handshake to the member** — one file set:
`crates/nvs-db/src/conn.rs` and the drivers' handshake readers, then
`crates/nvs-stdlib/src/db/registry.rs` and `crates/nvs-stdlib/tests/db_stream.rs`. The standing
decision is the goal's own § *Standing decisions*: the server's own string, kept from what the
handshake already delivered, and no driver issues `SELECT version()` for it.

- [ ] **Each driver keeps the version its handshake sent**, and the four in-crate tests the goal
      names assert it — `postgres_keeps_server_version_from_its_parameter_status`,
      `mysql_keeps_the_greetings_version_string`, `tds_keeps_the_version_its_loginack_sent` and
      `sqlite_answers_its_library_version`. The field belongs beside the state on each connection:
      `crates/nvs-db/src/conn.rs:751` (`PgConn`), `crates/nvs-db/src/conn.rs:946` (`TdsConn`) and
      `crates/nvs-db/src/conn.rs:1008` (`SqliteConn`). MySQL already parses a `(u16, u16, u16)` out
      of the greeting at `crates/nvs-db/src/mysql.rs:821`, which is numbers where the decision is the
      string the server sent — keep both or keep the string, but the member answers the string.
- [ ] **`Core\Db\Connection::serverVersion` over it**, with no round trip. The known gap that names
      what it was waiting on is `crates/nvs-stdlib/src/db/mod.rs:284`, and the card goes in
      `BEYOND_QUERYABLE` at `crates/nvs-stdlib/src/db/registry.rs:219`.
      `server_version_answers_what_the_connection_kept` joins
      `crates/nvs-stdlib/tests/db_stream.rs:1`, whose `leg()` already opens a connection per driver.
- [ ] **The conformance case**, `tests/conformance/core/db-server-version-answers-what-the-connection-kept.nvst`
      — a `:memory:` block reaches SQLite alone, so what it pins is the member answering the library
      version rather than a version table. `rule:core-classes/db-one-api` is the rule it cites, and
      `tests/conformance/core/db-a-sqlite-block-opens-a-file-and-runs-statements-over-it.nvst:1` is
      the block and the capability line to copy.

## Backlog

- `streamAs<T>`, stage 5's other half, including the compile-time refusal case under
  `tests/conformance/reject/` — `docs/agent/loop-goal.toml:10563`.
- The spec roster beyond `Queryable`, `crates/nvs-stdlib/tests/spec_registry_coverage.rs`, which
  stage 5's third check names.
- The matrix's socket leg is published by nothing, so `db_stream`'s `Location::Socket` arm skips
  without asserting — `crates/nvs-db/src/matrix.rs:43`, known gap 1, `unowned`.
