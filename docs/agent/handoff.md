# Handoff

## State

**Goal `sqlite-queue`, stages 2 and 3 are landed.** The goal's one ADR slot is taken by
[0170](../decisions/0170.md), which records that on this backend the claim's mutual exclusion *is*
the immediate transaction; `rule:concurrency/claiming-is-one-statement`'s fragment now carries that
sentence and its `because` names the record.

- **The worker half of § 4's statements is complete on SQLite.** `INSERT_SQLITE` is at
  `crates/nvs-stdlib/src/queue.rs:684` and is its own text — the dedupe read carries no `for
  update`, because the immediate transaction took the write lock before it ran. `DEAD_LETTER_SQLITE`,
  `QUEUES_SQLITE`, `SUCCEEDED_SQLITE` and `RETRY_SQLITE` are MySQL's own constants **aliased rather
  than copied**; `DEAD_LETTER_SQLITE`'s doc owns why, and the day a change belongs to one backend
  alone is the day one of them becomes its own literal.
- `crates/nvs-stdlib/tests/queue_sqlite.rs` runs six cases on the default `verify.py` legs, three of
  them new: the insert's pair over two connections, the key released by a claim, and the keyless
  push that collides with nothing.
- **Nothing past the statements exists.** `Queued` at `crates/nvs-stdlib/src/queue.rs:2291` still has
  two arms, so `Core\Queue` refuses a SQLite block at `no_dialect` and `nvs serve` starts no worker
  for one. `no_dialect`'s SQLite sentence now says the statements are *complete for PostgreSQL and
  MySQL only*, which is true until stage 4 lands.
- Nothing is blocked, and no design call is waiting on the user.

## Next group

**Stage 4: the six members' statements** — one file set: `crates/nvs-stdlib/src/queue.rs` and
`crates/nvs-stdlib/tests/queue_sqlite.rs`. The order below is the goal's own § *Stage 4*.

- [ ] **`STATUS_SQLITE`, `CANCEL_SQLITE` and `COUNTS_SQLITE`** — beside their twins at
      `crates/nvs-stdlib/src/queue.rs:975`, `crates/nvs-stdlib/src/queue.rs:1011` and
      `crates/nvs-stdlib/src/queue.rs:1225`, under `rule:concurrency/queue-four-members`. All three
      are portable: check whether MySQL's text is byte-identical here too, and alias it the way
      `DEAD_LETTER_SQLITE` does rather than transcribing it. The state ordinals stay literals for
      `PENDING`'s reason, and `queue_statements_agree_with_the_state_enum` gains the third list
      rather than a second test beside it.
- [ ] **`DELETE_SQLITE`, the one member with no single-statement shape here** — `DELETE_MYSQL` at
      `crates/nvs-stdlib/src/queue.rs:1081` is a multi-table `delete j, d` SQLite cannot spell, and
      PostgreSQL's is a data-modifying CTE it also cannot. So it is a `Split` of two deletes inside
      stage 2's transaction, under `rule:concurrency/queue-deletion-is-explicit-and-bounded`, and
      `DELETE_MYSQL`'s doc is the argument for why the two must be one moment.
- [ ] **`PURGE_SQLITE` and `PURGE_DEAD_SQLITE` copy PostgreSQL's bound, not MySQL's** —
      `crates/nvs-stdlib/src/queue.rs:1172` and `crates/nvs-stdlib/src/queue.rs:1181`. `delete …
      limit` needs a build option `libsqlite3-sys`'s bundled build does not set, and `delete … where
      id in (select id … order by id limit ?)` needs none. The bound is the rule and the spelling is
      not, per the goal's § *Standing decisions*.
- [ ] **The cases for whichever of those runs a construct rather than a copy** —
      `crates/nvs-stdlib/tests/queue_sqlite.rs:292`'s `push_in_two` is the shape: the statement run
      for real over two connections, asserted on both sides of its bound.

## Backlog

- Stage 5's three seams: `Queued`'s third arm at `crates/nvs-stdlib/src/queue.rs:2291`, `counted_row`
  at `crates/nvs-stdlib/src/queue.rs:2767`, `runs`, `no_dialect`'s SQLite arm, and the worker's
  `Wire` and local `Dialect` — `docs/agent/loop-goal.md` § *Stage 5*.
- Stage 6: `examples/queue-sqlite.nvs` (the acceptance check that is red today), the matrix leg and
  the reference page — `docs/agent/loop-goal.md` § *Stage 6*.
- `SqliteConn::begin_immediate` refuses any depth above zero, so a member reached inside a caller's
  own transaction cannot ask for one — `crates/nvs-db/src/sqlite.rs:763` and ADR 0170 § 3.
