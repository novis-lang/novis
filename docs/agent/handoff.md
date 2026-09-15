# Handoff

## State

**Goal `m8-db-queue`, stages 0, 2 and 3 are done, and stage 4 is half of the way.** Three drivers park
a read: `crates/nvs-db/src/mysql.rs`'s `MySqlCursor` for MySQL and MariaDB, and now
`crates/nvs-db/src/tds/rows.rs`'s `TdsCursor` for SQL Server. `TdsRows` is the cursor beside a borrow,
`next_row_of` is the free reader both paths drive, `Walk` is the per-step group of wire, state, cache
and cursor, and `TdsConn` carries `stream`/`stream_next_row`/`stream_columns`/`stream_span`/
`name_stream_connection`/`end_stream` over a `reading: Option<TdsCursor>`. § 13's reset drains the
parked walk before `sp_reset_connection` goes out.

**§ 1's cache could not travel with a parked cursor**, which is the one thing this protocol cost:
`PendingPlan` is the key on the cursor and the cache is handed to the step that files it
(`crates/nvs-db/src/tds/rows.rs` `PendingPlan` owns the reasoning).

**SQLite is the driver left, and `Core\Db\Connection::stream` still answers on three.** Nothing is
blocked; stage 1 is the carried floor and the driver's to run.

## Next group

**Stage 4: SQLite's pinned walk, then the stdlib runs out of drivers** — one file set:
`crates/nvs-db/src/sqlite.rs`, `crates/nvs-db/src/conn.rs`, `crates/nvs-host/src/blocking.rs`,
`crates/nvs-stdlib/src/db/stream.rs` and `crates/nvs-stdlib/src/db/registry.rs`.
`rule:core-classes/a-stream-parks-its-read-on-the-connection`'s SQLite paragraph is the user's
pinned-thread call and what both items must satisfy; `docs/decisions/0187.md` § 1 is the reasoning.

- [ ] **SQLite parks a walk on one pinned thread.** `crates/nvs-db/src/sqlite.rs:595` `query` hands
      one closure to the blocking pool and gets the whole result set back, and
      `crates/nvs-db/src/sqlite.rs:33-40`'s module doc is written around that being the trade. A walk
      instead holds a pool thread that owns the `rusqlite::Statement` and answers a row per request.
      The primitive to choose between is `crates/nvs-host/src/blocking.rs:320` `run`, which is
      one closure and its answer, and `crates/nvs-host/src/blocking.rs:161` `BlockingPool::submit`;
      `crates/nvs-host/src/blocking.rs:76` `bound` is the cap a walk holding a thread eats into, so
      say in the module doc what an exhausted pool does. It parks on
      `crates/nvs-db/src/conn.rs:1008` `SqliteConn`, beside the members `TdsConn` now carries, and
      is released when the walk ends **or its task does**.
- [ ] **The two stdlib arms, and `unstreamed` runs out of drivers.**
      `crates/nvs-stdlib/src/db/stream.rs:291` `stream_over` and
      `crates/nvs-stdlib/src/db/stream.rs:234` `mysql_step` are the shape a SQL Server and a SQLite
      step take; `crates/nvs-stdlib/src/db/stream.rs:402` `unstreamed` and `registry.rs`'s
      `RuntimeError` card name the two that are left and must name none.

## Backlog

- `crates/nvs-stdlib/tests/db_stream.rs` and `tools/db-matrix.py:115` `SUITES`, plus
  `tests/conformance/core/db-stream-on-sqlite-walks-its-rows-or-refuses-naming-the-driver.nvst` —
  stage 4's second and third checks, due once all five drivers park a read.
- PostgreSQL's `reset` refuses a busy connection (`crates/nvs-db/src/pg.rs:3236`) where MySQL's and
  now SQL Server's drain the parked walk first; decide whether `PgConn::reset` should drain too, in
  `rule:core-classes/db-connection-busy-state`'s terms.
- `crates/nvs-stdlib/src/db/mod.rs` gap 3 still owes `streamAs` and `serverVersion` — stage 5's.
- `crates/nvs-server/src/schedule.rs:1596`
  `a_fleet_lease_is_renewed_while_its_run_is_in_flight` failed once beside the other test binaries
  (1 renewal where it wants 2, over a 130 ms run renewed every 20 ms) and passed alone and on the
  next full run — the playbook's *failed beside … passed alone* bullet is what it owes, and its
  remedy is a commit of nvs-server's own.
- An abandoned `TdsRows` still leaves the connection in `State::Streaming` with no `Drop` to drain
  it, where `MySqlRows` and `PgRows` drain on drop; `TdsConn::end_stream` covers the parked walk
  only.
