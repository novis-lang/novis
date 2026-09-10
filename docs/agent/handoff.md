# Handoff

## State

**Goal `sqlite-queue`, stages 2, 3 and 4 are landed: every statement the queue sends exists for
SQLite.** The goal's one ADR slot is taken by [0170](../decisions/0170.md), and
`rule:concurrency/claiming-is-one-statement`'s fragment carries the immediate transaction.

- **The six members' texts are in `crates/nvs-stdlib/src/queue.rs` beside their twins.**
  `STATUS_SQLITE`, `CANCEL_SQLITE` and `COUNTS_SQLITE` are MySQL's own constants aliased, as the
  worker's four are; `DEAD_LETTER_SQLITE`'s doc owns why an alias and not a copy. `DELETE_SQLITE` is
  a `Split` of two deletes — the first `Split` whose second statement is not keyed on the first's
  row, which `Split`'s field docs now state — and `PURGE_SQLITE`/`PURGE_DEAD_SQLITE` are their own
  literals: `PURGE_POSTGRES`'s subquery bound with `PURGE_MYSQL`'s placeholder order.
- `crates/nvs-stdlib/tests/queue_sqlite.rs` runs 12 cases on the default `verify.py` legs, six of
  them new — one per member, plus the delete's two arms and both purges' options.
- **Nothing routes a call to any of them.** `runs` at `crates/nvs-stdlib/src/queue.rs:2369` answers
  `false` for SQLite, `Queued` at `crates/nvs-stdlib/src/queue.rs:2422` has two arms, and
  `purge_texts` at `crates/nvs-stdlib/src/queue.rs:3328` hands back a `(postgres, mysql, table)`
  triple that stage 5 has to widen. `no_dialect`'s SQLite sentence now says the statements are
  written and nothing carries a call to one yet.
- Nothing is blocked, no design call is waiting on the user, and no `[context]` field was missing
  from this session's pack.

## Next group

**Stage 5: the three seams** — one file set: `crates/nvs-stdlib/src/queue.rs` and
`crates/nvs-cli/src/worker.rs`. The order below is the goal's own § *Stage 5*, with `purge_texts`
added because it is a two-dialect shape rather than a two-dialect list.

- [ ] **`runs` gains SQLite and `no_dialect` loses it** — `crates/nvs-stdlib/src/queue.rs:2369` and
      `crates/nvs-stdlib/src/queue.rs:2451`, under `rule:core-classes/db-drivers-are-an-enum`. The
      refusal test at `crates/nvs-stdlib/src/queue.rs:3642` reads `runs` rather than a second list
      and its SQL Server assertion is written to survive this goal, so it moves with the seam. The
      SQL Server arm stays spelled.
- [ ] **`Queued` gains a third arm of its own rather than joining `Framed`** —
      `crates/nvs-stdlib/src/queue.rs:2422`, handed back by `queue_connection` at
      `crates/nvs-stdlib/src/queue.rs:2390`, under `rule:core-classes/db-drivers-are-an-enum`.
      `SqliteConn::query` takes an owned `Vec<SqliteValue>` where the framed drivers take
      already-encoded wire bytes, and `crates/nvs-db/src/sqlite.rs:304`'s `encode` is the
      conversion — a type and not a dialect, which is why it cannot be flattened the way MariaDB is.
- [ ] **`purge_texts` carries two texts and a table, and needs the third text** —
      `crates/nvs-stdlib/src/queue.rs:3328`, under
      `rule:concurrency/queue-deletion-is-explicit-and-bounded`. It is the one seam the goal's stage
      list does not name; whichever shape it takes, *which table a selection reads* stays the fact
      its doc says a statement's own text cannot state.
- [ ] **The worker's `Wire` and `Dialect` gain a third arm** — `crates/nvs-cli/src/worker.rs:932`
      and `crates/nvs-cli/src/worker.rs:959`, with `open` at `crates/nvs-cli/src/worker.rs:866`
      keeping SQL Server alone in its refusal, under
      `rule:concurrency/claiming-is-one-statement`. Every `Split` it sends runs inside
      `begin_immediate`, per the goal's § *Standing decisions*.

## Backlog

- Stage 6: the SQLite leg of `tools/db-matrix.py:165` runs a suite list this goal extends — goal
  `35-sqlite-queue.md` § *Stage 6*.
- Stage 6: `crates/nvs-stdlib/tests/queue.rs:313` holds its own two-armed `Dialect`, beside the
  worker's — same § *Stage 6*.
- Stage 6: the conformance cases for the members, and `examples/queue-sqlite.nvs`, which the
  driver's acceptance check names and which is a file of its own beside the frozen
  `examples/queue.nvs`.
- `[queue] workers` owes the sentence saying where the number stops buying throughput on a
  single-writer backend — goal § *Standing decisions*, not yet written anywhere an operator reads.
