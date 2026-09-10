# Handoff

## State

**Goal 35 — the queue runs on SQLite — has just started; nothing of it has landed yet.** Goal
`queue-purge`'s whole list is this goal's Stage 1 floor.

What is settled before the first session:

- **The mechanism is not a choice.** `rule:concurrency/claiming-is-one-statement` names it — "an
  immediate transaction on SQLite, whose single-writer model makes contention moot" — and the goal
  implements it rather than re-opening it. A session that prefers a single `update … where id =
  (select …) returning …` has found a real alternative and still may not take it here: that is an
  edit to the fragment through a record whose `changes:` block names it.
- **The mechanism does not exist yet, and that is the whole keystone.** `SqliteConn::begin` emits a
  bare `BEGIN` at depth 0 and ignores its `isolation` argument entirely, which SQLite reads as
  `DEFERRED`. A deferred transaction that reads and then writes is answered with `SQLITE_BUSY` on
  the upgrade *without honouring the busy timeout*, so `CLAIM_MYSQL`'s shape transcribed onto this
  backend is a claim that fails only under two connections and never under one.
- **`Core\Db`'s SQLite support is complete and always was.** The gap is `Core\Queue`'s § 4 texts
  alone. `crates/nvs-cli/src/worker.rs:903-911` says otherwise to an operator's face and is stage
  0's first correction.
- **SQL Server stays out.** Its refusal is a schema question in front of a statement question, and
  `no_dialect`'s SQL Server arm keeps its "filtered index" sentence exactly as it is.

## Next group

**Stage 2: the immediate transaction, and the claim on top of it** — one file set:
`crates/nvs-db/src/sqlite.rs`, `crates/nvs-stdlib/src/queue.rs`.

- [ ] **The primitive** — `crates/nvs-db/src/sqlite.rs:@begin`, whose depth-0 arm is the bare
      `BEGIN`. A dedicated entry point rather than a new `Isolation` case, so
      `Core\Db::transaction`'s behaviour does not move for any existing caller. Asking for one at a
      depth above zero is the refusal to write, beside the two that method already has.
- [ ] **The depth accounting** — the same file's `commit` deliberately leaves the depth where it was
      on a refused outermost commit. An immediate transaction is an outermost one by definition, so
      the new path must not invent a second rule for the counter.
- [ ] **`CLAIM_SQLITE`** — `crates/nvs-stdlib/src/queue.rs:@CLAIM_MYSQL` is the shape: two statements
      inside the transaction, answering `CLAIM_POSTGRES`'s six columns in its order, with
      `attempts + 1 as attempts` for that constant's reason. **No locking clause**, and the comment
      says why — the single writer is the mutual exclusion, and a later reader will otherwise try to
      add `skip locked`.
- [ ] **The check is two connections over one file.** A claim proven on one connection is a claim
      whose whole failure mode was never exercised.
- [ ] **The ADR** — this goal's one slot, taken here: the claim, the primitive, and where the
      primitive sits.
- [ ] **Stage 0's first correction**, since the file is open anyway:
      `crates/nvs-cli/src/worker.rs:903-911` is wrong about `Core\Db` today.

## Backlog

- **Stage 3 — the worker's other statements** (`crates/nvs-stdlib/src/queue.rs` alone): `INSERT`,
  `DEAD_LETTER`, `SUCCEEDED`, `RETRY`, `QUEUES`. Cheap to take in the same session as stage 2 if the
  budget holds — it is the same file and the same transaction.
- **Stage 4 — the six members' statements** (`crates/nvs-stdlib/src/queue.rs` alone): `STATUS`,
  `CANCEL`, `COUNTS`, `DELETE`, `PURGE`, `PURGE_DEAD`. `DELETE` is a `Split`; `PURGE` copies
  PostgreSQL's subquery bound, never `delete … limit`.
- **Stage 5 — the three seams** (`queue.rs` + `crates/nvs-cli/src/worker.rs`): `runs`, `Queued`,
  `no_dialect`, `Wire`, the worker's local `Dialect`, and `open`'s refusal arm.
- **Stage 6 — the matrix leg and the cases** (`crates/nvs-stdlib/tests/queue.rs`,
  `tools/db-matrix.py`, `tests/conformance/`): the first queue coverage that runs on a machine with
  no daemon.
- When this goal's last check goes green the driver takes goal `serve-runs-the-queue`.
