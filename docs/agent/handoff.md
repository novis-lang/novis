# Handoff

## State

**Goal `m8-db-queue`, stage 4 is landed on the driver side: every driver parks a read, and
`Core\Db\Connection::stream` answers on all five.** SQLite's walk is a pool thread rather than a
message boundary — `crates/nvs-db/src/sqlite.rs`'s `SqliteCursor` parks the handle, `walk` is the
closure that owns the lock, the statement and the cursor, and one owned row crosses back per step.
The primitive under it is new: `nvs_host::blocking::pin` hands a pool thread a closure that stays on
it and answers one `Pinned::ask` at a time, which is `run`'s park per question rather than per call.

**A deadline filed mid-walk goes through the walk's own thread** (`set_busy_timeout`), because the
walk holds the lock on the connection and taking it on the core would wedge the worker. That is the
one place this driver's shape leaks into a call that is not about streaming.

`nvs-stdlib`'s two arms landed with it, so `unstreamed` and its `RuntimeError` reference-card row are
gone. What stage 4 still owes is proof, not code: the `db_stream` suite, the `.nvst` case and the
five-driver matrix run. Nothing is blocked; stage 1 is the carried floor and the driver's to run.

## Next group

**Stage 4: the proofs `stream` now owes** — one file set: `crates/nvs-stdlib/tests/db_stream.rs`
(new), `tools/db-matrix.py` and
`tests/conformance/core/db-stream-on-sqlite-walks-its-rows-or-refuses-naming-the-driver.nvst` (new).
`rule:core-classes/db-streaming` is the contract all three assert; the mechanism under it is
`rule:core-classes/a-stream-parks-its-read-on-the-connection`.

- [ ] **The `db_stream` suite, gated on `NVS_DB_MATRIX_DRIVER`.** Three tests the goal names:
      `stream_answers_on_every_driver_or_names_its_recorded_refusal`,
      `stream_holds_one_row_and_not_the_result_set_on_every_driver` and
      `a_second_statement_on_a_streaming_connection_is_a_logic_error_on_every_driver`. The gate's
      shape — a case that finds the variable unset asserting nothing — is
      `crates/nvs-db/src/matrix.rs:33`, and `crates/nvs-stdlib/tests/queue.rs:1` is the sibling
      suite that already runs this way. The one-row assertion is the only one with no precedent:
      `crates/nvs-stdlib/src/db/stream.rs:24` `stream_answers_rows_without_holding_the_result_set`
      counts the parked row over a thousand rows and is what to count against.
- [ ] **`db-matrix.py` runs it.** `tools/db-matrix.py:115` `SUITES` is the tuple every driver runs;
      `tools/db-matrix.py:129` `NO_SERVER_SUITES` is SQLite's extra, and a stream suite belongs in
      the first.
- [ ] **The conformance case, over a `:memory:` block.** A `.nvst` case reaches SQLite alone
      (the goal's standing decisions), so it walks rows and asserts the second-statement refusal.
      `tests/conformance/core/db-stream-refuses-a-tainted-statement.nvst:1` is the sibling to copy
      the block and the header from, and the playbook's *a `close`d SQLite `:memory:` connection*
      bullet is the trap it sits next to.

## Backlog

- Concurrent SQLite walks can hold every thread of a worker's blocking pool (`bound` is twice the
  core count); nothing caps walks below it, and what bounds one is the request that opened it —
  `nvs_host::blocking::pin`'s own docs.
- PostgreSQL's `reset` refuses a busy connection (`crates/nvs-db/src/pg.rs:3236`) where MySQL's,
  SQL Server's and now SQLite's end the parked walk first; decide whether `PgConn::reset` should
  too, in `rule:core-classes/db-connection-busy-state`'s terms.
- `crates/nvs-stdlib/src/db/mod.rs` gap 3 still owes `streamAs` and `serverVersion` — stage 5's.
- An abandoned `TdsRows` still leaves the connection in `State::Streaming` with no `Drop` to drain
  it, where `MySqlRows` and `PgRows` drain on drop; `TdsConn::end_stream` covers the parked walk only.
- `crates/nvs-server/src/schedule.rs:1596` `a_fleet_lease_is_renewed_while_its_run_is_in_flight`
  failed once beside the other test binaries and passed alone — the playbook's *failed beside …
  passed alone* bullet is what it owes.
