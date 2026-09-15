# Handoff

## State

**Goal `m8-db-queue`, stages 0, 2 and 3 are done.** The parked read is on the MySQL wire:
`crates/nvs-db/src/mysql.rs`'s `MySqlCursor` is the read state split off `MySqlRows`' borrow,
`next_row_of` is the one row reader both paths drive, and `MySqlConn`/`MariaConn` carry
`stream`/`stream_next_row`/`stream_columns`/`stream_span`/`name_stream_connection`/`end_stream` over
a `reading: Option<MySqlCursor>` field. Both resets drain the parked walk before
`COM_RESET_CONNECTION` goes out. Stage 3's three named tests pass against a scripted server.

**`Core\Db\Connection::stream` now answers on three drivers.** `crates/nvs-stdlib/src/db/stream.rs`'s
`mysql_step` and `stream_over` run the walk over `Framed`, which grew the five stream members;
`unstreamed` and `registry.rs`'s `RuntimeError` card name SQLite and SQL Server as what is left.

**That stdlib half has no runtime proof yet** — no `.nvst` case can reach a MySQL server, and stage 4's
`db_stream` suite is where it meets one. Nothing is blocked; stage 1 is the carried floor and the
driver's to run.

## Next group

**Stage 4: the other two drivers park a read** — one file set: `crates/nvs-db/src/tds/rows.rs`,
`crates/nvs-db/src/tds/mod.rs`, `crates/nvs-db/src/sqlite.rs` and `crates/nvs-db/src/conn.rs`.
`rule:core-classes/a-stream-parks-its-read-on-the-connection` is what both must satisfy —
including its SQLite paragraph, which is the user's pinned-thread call — and
`docs/decisions/0187.md` § 1 is the reasoning. `crates/nvs-db/src/mysql.rs:3239` `MySqlCursor` is the
shape to copy, with `crates/nvs-db/src/mysql.rs:3390` `next_row_of` as the single row reader.

- [ ] **TDS parks its read.** `crates/nvs-db/src/tds/rows.rs:39` `TdsRows` holds the wire and the read
      state together; split the state off and park it on `crates/nvs-db/src/conn.rs:946` `TdsConn`,
      with `crates/nvs-db/src/tds/rows.rs:177` `next_row` becoming the free reader both paths drive.
      `COLMETADATA` is what the cursor carries (`rule:core-classes/a-stream-parks-its-read-on-the-connection`).
      Prove it against the scripted server `crates/nvs-db/src/tds/stream.rs`'s cases already use:
      `tds_stream_parks_its_read_and_answers_one_row_per_step`.
- [ ] **SQLite parks a walk on one pinned thread.** `crates/nvs-db/src/sqlite.rs:595` `query` takes the
      core back with the rows in hand; a walk instead holds one thread from `nvs-host`'s blocking pool
      for its life, released when the walk is drained, dropped or its task ends
      (`rule:core-classes/a-stream-parks-its-read-on-the-connection`, the SQLite paragraph, and the
      goal's standing decision). `crates/nvs-db/src/conn.rs:997` `SqliteConn` is where the handle sits.
      Test: `sqlite_stream_parks_its_read_or_names_its_recorded_refusal`.
- [ ] **The two stdlib arms, and `unstreamed` runs out of drivers.**
      `crates/nvs-stdlib/src/db/stream.rs:134` `stream_step` gains a `Tds` and a `Sqlite` arm beside
      `mysql_step`, and `crates/nvs-stdlib/src/db/stream.rs:402` `unstreamed` plus
      `crates/nvs-stdlib/src/db/registry.rs:2322`'s `RuntimeError` card stop naming a driver that has
      no parked read (`rule:core-classes/db-streaming`).

## Backlog

- `crates/nvs-stdlib/tests/db_stream.rs` and `tools/db-matrix.py:115` `SUITES`, plus
  `tests/conformance/core/db-stream-on-sqlite-walks-its-rows-or-refuses-naming-the-driver.nvst` —
  stage 4's second and third checks, due once all five drivers park a read.
- PostgreSQL's `reset` refuses a busy connection (`crates/nvs-db/src/pg.rs:3236`) where MySQL's now
  drains the parked walk first; decide whether `PgConn::reset` should drain too, in
  `rule:core-classes/db-connection-busy-state`'s terms.
- `crates/nvs-stdlib/src/db/mod.rs` gap 3 still owes `streamAs` and `serverVersion` — stage 5's.
