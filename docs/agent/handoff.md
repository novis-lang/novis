# Handoff

## State

**Goal `m8-db-queue`, stage 7's first check is closed.** A unique key over a nullable column is
SQL Server's filtered index at both doors: `CREATE UNIQUE INDEX … WHERE <column> IS NOT NULL`,
written beside the `CREATE TABLE` rather than as a clause of it, and in place of the
`ADD CONSTRAINT` after the fact (`crates/nvs-db/src/ddl.rs:227`). The predicate names the key's
nullable columns alone, so a key every column of which is `not null` keeps the constraint form on
all four dialects. `rule:core-classes/a-unique-key-reads-nulls-as-distinct` is the whole semantics
and it was already written; no grade moved, because `rule:core-classes/schema-plan` already grades
a dialect's spelling as the construct.

The catalog reads that index back as the key that asked for it
(`crates/nvs-db/src/catalog.rs:1023`), matched in the assembly rather than in the statement because
the match needs the index's columns and their nullability together. `Read::Indexes` therefore
carries `nvs_filter`, which all four dialects answer — three of them as a typed null — so
`IndexRow` has a seventh field that `crates/nvs-db/src/direct.rs:133` and
`crates/nvs-stdlib/src/db/schema.rs:473` fill.

**The group's third slice is not owed.** `plan`'s refusal is the fallback for a round trip that
does not close, and it closes: `a_filtered_unique_index_reads_back_as_the_same_key` is the unit
proof, and `assembled_fixture` gained a nullable unique column, so the four matrix round trips ask
a real server the same question under `python tools/db-matrix.py --all`.

`Core\Queue`'s `nvs_jobs_dedupe` is one of those keys, so on SQL Server it lands as a statement
beside the `CREATE TABLE` rather than a clause inside it — which is what a SQL Server queue text
has to expect of the schema it converges (`crates/nvs-stdlib/src/queue.rs:5455`).

Stage 7's other two checks are open, which is the ordinary state: `nvs_stdlib::queue` has no SQL
Server statement roster at all, and a dead-lettered row carries one attempt's error rather than
every attempt's.

## Next group

**Stage 7: the queue's SQL Server half** — one file set: `crates/nvs-stdlib/tests/queue.rs` and
`crates/nvs-stdlib/src/queue.rs`. The DDL under it is landed, so what is left is the statement
roster and the harness that runs it. `rule:core-classes/queue-storage-is-a-table` owns the schema
and `rule:concurrency/enqueue-commits-with-your-write` the transactional half.

- [ ] **`every_statement_the_queue_sends_has_a_sql_server_text` does not exist**, and stage 7's
      `-p nvs-stdlib --test queue` check names it. The roster is one constant per dialect per
      statement from `crates/nvs-stdlib/src/queue.rs:423`, and `nvs_stdlib::queue::runs`
      (`crates/nvs-stdlib/src/queue.rs:2373`) is what dispatches on the dialect. The schema value
      at `crates/nvs-stdlib/src/queue.rs:353` already emits on SQL Server through
      `nvs_db::ddl`, filtered index included.
- [ ] **`queue_runs_on_every_driver` does not exist.** The test harness refuses SQL Server before
      it connects: `Conn` has three arms (`crates/nvs-stdlib/tests/queue.rs:223`) and `endpoint`
      skips the driver, which `crates/nvs-stdlib/tests/queue.rs:210` spells as an `unreachable!`
      rather than a `_` so the arm becomes a build failure the day a schema exists. A `TdsConn`
      arm and its `Dialect` are what that item is.
- [ ] **`an_enqueue_commits_with_the_write_on_sql_server` does not exist.** The framed cases are
      the shape to follow — `crates/nvs-stdlib/tests/queue.rs:2834` is the same claim on the
      dialects that have one — and TDS's transaction control is what it exercises.

## Backlog

- Stage 7's `errors` array, bounded by attempts × the capped message
  (`crates/nvs-stdlib/src/queue.rs:882`), with `a_dead_lettered_row_carries_every_attempts_error`
  and `a_worker_opens_a_sql_server_queue_block` under `-p nvs-cli`
  (`crates/nvs-cli/src/worker.rs:1883`). A file set of its own, so a group of its own.
- Dropping a filtered unique key on SQL Server still emits `ALTER TABLE … DROP CONSTRAINT`
  (`crates/nvs-db/src/ddl.rs:748`), which that server refuses for an index. `Change::DropKey`
  carries the after-table and the doomed key's name alone, so the emitter cannot tell the two
  spellings apart; it is a report that is never applied, and § 8 already shows a step that will
  not run. Owner: `crates/nvs-db/src/ddl.rs`'s module doc.
- The pack's `[context] modules` names neither `crates/nvs-db/src/plan.rs`,
  `crates/nvs-db/src/schema.rs` nor `crates/nvs-db/src/direct.rs`, and all three were read this
  session — `Change`'s variants, `Key`/`Column`'s accessors and the row reader are where a DDL or
  catalog slice goes next.
- A closure reusing an enclosing `foreach`'s variable name is an ICE, not a diagnostic —
  `crates/nvs-ir/src/lower/expr.rs:2667`; the playbook bullet is the workaround, and the fix is
  `nvs_types`' capture set. Owner: `crates/nvs-ir`'s module doc.
- `nvs_types::derive` gap 2's subject, the `CodecTy::Opaque` field the reader has no case for, is
  gap 1's erasure and still open. Owner: `crates/nvs-types/src/derive.rs`'s module doc.
