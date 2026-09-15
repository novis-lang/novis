# Handoff

## State

**Goal `m8-db-queue`, stage 7. `Core\Queue` sends its statements over all five backends.**
`queue::runs` (`crates/nvs-stdlib/src/queue.rs:2640`) answers `true` for every driver, because
`Queued::SqlServer` is `queue_connection`'s fourth arm and `tds_counted`
(`crates/nvs-stdlib/src/queue.rs:3608`) is `counted_row`'s fourth send path — so `push`, `status`,
`cancel`, `delete`, `purge` and `stats` all reach SQL Server. The enqueue is a pair inside a
transaction (`push_in_tds`, `crates/nvs-stdlib/src/queue.rs:3005`) and reads its id off
`output inserted.id`, `TdsRows` having no `last_id` at all.

**`no_dialect` is deleted rather than narrowed.** Every driver has an arm, so a sixth is a build
failure at `queue_connection` instead of a sentence a session has to keep true, and the two unit
tests over that refusal went with it. `queue_runs_on_every_driver`
(`crates/nvs-stdlib/tests/queue.rs:1162`) is what holds the predicate to the roster now: it asks
`queue::texts` for every driver's member set and fails a driver `runs` claims but the texts skipped.

**Two things are still unsent.** `nvs queue work` has no SQL Server arm, so a job pushed onto that
backend is kept and never claimed — the refusal an operator reads says that now, and names this
command rather than the queue's statements. And no case in `crates/nvs-stdlib/tests/queue.rs` can
open a SQL Server connection, so nothing in the group below has met a real server yet; the matrix's
`mssql` leg runs the two caseless tests and skips every other one.

## Next group

**Stage 7: the SQL Server leg, and the worker behind it** — one file set:
`crates/nvs-stdlib/tests/queue.rs` and `crates/nvs-cli/src/worker.rs`. The send path is landed, so
what is left is the harness that watches it and the command that claims a job back.
`rule:concurrency/enqueue-commits-with-your-write` owns the transactional half and
`rule:concurrency/claiming-is-one-statement` the claim.

- [ ] **`an_enqueue_commits_with_the_write_on_sql_server` does not exist**, and the harness refuses
      before it can. `Conn` (`crates/nvs-stdlib/tests/queue.rs:228`) has three arms and `Dialect`
      (`crates/nvs-stdlib/tests/queue.rs:319`) two, so a `Conn::SqlServer` needs its arm in
      `dialect`, `driver`, `text` (`@p1`), `begin`/`commit`/`roll_back` and `depth`, a `TdsTarget`
      arm in `open` (`crates/nvs-stdlib/tests/queue.rs:175`), and a rendering arm in `rows`
      (`crates/nvs-stdlib/tests/queue.rs:514`) over `nvs_db::tds::scalar` — which is where the work
      is, the other two arms rendering `PgScalar` and `MySqlScalar`. Then a `sqlserver()` gate beside
      `framed()` (`crates/nvs-stdlib/tests/queue.rs:117`) and the case itself, in
      `an_enqueue_commits_with_the_write_that_made_it`'s shape.
- [ ] **`a_worker_opens_a_sql_server_queue_block` does not exist**, and `open`
      (`crates/nvs-cli/src/worker.rs:1202`) still answers `None` for the driver. It needs a `Wire`
      arm (`crates/nvs-cli/src/worker.rs:1311`) and a `Dialect` one
      (`crates/nvs-cli/src/worker.rs:1348`) sending `CLAIM_SQLSERVER`, `SUCCEEDED_SQLSERVER`,
      `RETRY_SQLSERVER` and `DEAD_LETTER_SQLSERVER` — all four are in `queue::texts` already — and
      it deletes `sql_server_gap` (`crates/nvs-cli/src/worker.rs:1266`) and the test over it
      (`crates/nvs-cli/src/worker.rs:1614`) the way this session deleted `no_dialect`.

## Backlog

- `a_dead_lettered_row_carries_every_attempts_error` is stage 7's third check and is `-p nvs-cli`'s
  — `docs/agent/loop-goal.toml:10669`.
- The `unowned`-tagged gaps in `crates/nvs-stdlib/src/queue.rs` and `crates/nvs-stdlib/src/db/mod.rs`
  are goal `unowned-closures`', per this goal's § *Standing decisions*.
- `crates/nvs-db/src/tds/rows.rs` has no `last_id`; nothing needs one while `output inserted.id`
  answers, and `push_in_tds`'s doc is the one home for why.
