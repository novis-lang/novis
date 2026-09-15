# Handoff

## State

**Goal `m8-db-queue`, stage 7. The queue's SQL Server statement roster is landed, and nothing
sends it yet.** Every member the queue has a statement for has a T-SQL text, and the one home for
what the queue sends is now `nvs_stdlib::queue::texts` (`crates/nvs-stdlib/src/queue.rs:1555`) —
`(member, sql)` for one driver, each `Split` flattened — which every dialect case asserts over in
place of a list of its own.

The texts take the shape the backend allows rather than one dialect's throughout. A common table
expression there updates but cannot insert, so the claim is one statement answering
`CLAIM_POSTGRES`'s list through `output inserted.<column>` while the push, the move and the delete
are pairs inside a transaction; the enqueue's dedupe read is `updlock, holdlock`, which is the range
lock an insert-if-absent needs where `for update` has no row to take. Every bound value reaches
`sp_prepexec` declared `nvarchar`, so each one a column types as a number is cast where it is
written, and `count`/`sum` are widened to `bigint` because they are `int` on this backend alone.

`runs` (`crates/nvs-stdlib/src/queue.rs:2649`) still answers `false` for SQL Server and
`queue_connection` has no arm for it, so the refusal an operator reads says the statements are
unsent rather than unwritten. The filtered-index half is on disk, so the schema converges there and
no doc waits on the vocabulary any more.

## Next group

**Stage 7: the queue's SQL Server send path** — one file set: `crates/nvs-stdlib/src/queue.rs` and
`crates/nvs-stdlib/tests/queue.rs`. The texts and the DDL under them are landed, so what is left is
the path that sends one and the harness that watches it.
`rule:core-classes/queue-storage-is-a-table` owns the schema and
`rule:concurrency/enqueue-commits-with-your-write` the transactional half.

- [ ] **`queue_runs_on_every_driver` does not exist**, and flipping `runs`
      (`crates/nvs-stdlib/src/queue.rs:2649`) is the last thing that slice does rather than the
      first. It needs a `Queued::SqlServer` variant (`crates/nvs-stdlib/src/queue.rs:2708`) and its
      arm in `queue_connection` (`crates/nvs-stdlib/src/queue.rs:2675`), a push beside
      `push_in_two` (`crates/nvs-stdlib/src/queue.rs:2987`) that reads the id off the row
      `output inserted.id` answers — `TdsRows` has no `last_id` at all, only `affected`
      (`crates/nvs-db/src/tds/rows.rs:292`) — and a fourth arm in `counted_row`
      (`crates/nvs-stdlib/src/queue.rs:3370`) for the five readers. `TdsConn::begin` and `commit`
      are `crates/nvs-db/src/tds/mod.rs:604` and `:636`, `query` is `:494`. It rewrites
      `runs_answers_false_for_sql_server_alone` (`crates/nvs-stdlib/src/queue.rs:4305`) and
      `no_dialect`'s SQL Server arm (`crates/nvs-stdlib/src/queue.rs:2741`) with it.
- [ ] **`an_enqueue_commits_with_the_write_on_sql_server` does not exist**, and the harness refuses
      the driver before the case can: `Conn` (`crates/nvs-stdlib/tests/queue.rs:223`) has no SQL
      Server arm, and neither `endpoint` (`crates/nvs-stdlib/tests/queue.rs:84`) nor `open`
      (`crates/nvs-stdlib/tests/queue.rs:172`) resolves one. The framed cases beside it
      (`crates/nvs-stdlib/tests/queue.rs:2903`) are the shape, and `python tools/db-matrix.py --all`
      is what points them at the server — which is also the first run the new texts meet one.

## Backlog

- Stage 7's `errors` array, bounded by attempts × the capped message
  (`crates/nvs-stdlib/src/queue.rs:882`), with `a_dead_lettered_row_carries_every_attempts_error`
  and `a_worker_opens_a_sql_server_queue_block` under `-p nvs-cli`
  (`crates/nvs-cli/src/worker.rs:1883`). A file set of its own, so a group of its own.
- `all_three_dialects_answer_a_claim_with_the_same_columns` still reads three: T-SQL names the
  claim's columns in an `output` clause rather than a `select` or a `returning`, so the fourth is a
  case beside it. The count in that name is pinned by a floor check, so the rename is the
  playbook's repair rather than a free edit. Owner: `crates/nvs-stdlib/src/queue.rs`.
- Dropping a filtered unique key on SQL Server still emits `ALTER TABLE … DROP CONSTRAINT`
  (`crates/nvs-db/src/ddl.rs:748`), which that server refuses for an index. `Change::DropKey`
  carries the after-table and the doomed key's name alone, so the emitter cannot tell the two
  spellings apart; it is a report that is never applied. Owner: `crates/nvs-db/src/ddl.rs`'s
  module doc.
- The pack's `[context] modules` names neither `crates/nvs-db/src/plan.rs` nor
  `crates/nvs-db/src/schema.rs`, and a DDL or catalog slice reads both.
- A closure reusing an enclosing `foreach`'s variable name is an ICE, not a diagnostic —
  `crates/nvs-ir/src/lower/expr.rs:2667`; the playbook bullet is the workaround, and the fix is
  `nvs_types`' capture set. Owner: `crates/nvs-ir`'s module doc.
- `nvs_types::derive` gap 2's subject, the `CodecTy::Opaque` field the reader has no case for, is
  gap 1's erasure and still open. Owner: `crates/nvs-types/src/derive.rs`'s module doc.
