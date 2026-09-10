# Handoff

## State

**Goal `sqlite-queue`, stage 5's stdlib half is landed: every `Core\Queue` member sends over a
SQLite connection.** `runs` at `crates/nvs-stdlib/src/queue.rs:2369` answers `true` for it,
`no_dialect` is SQL Server's alone, and `Queued` has a third arm. The goal's one ADR slot stays
taken by [0170](../decisions/0170.md).

- **The third arm is a binding and not a dialect.** `Queued::Sqlite` carries an
  `&mut nvs_db::SqliteConn`, and `Sent` at `crates/nvs-stdlib/src/queue.rs:2900` is what a member
  hands it: owned `nvs_db::SqliteValue`s in the framed dialect's order, built by a thunk so nothing
  is allocated on a connection that does not take them. `Sent::Pair` is `DELETE_SQLITE`, the one
  member with no single-statement shape.
- **`push`'s four string buffers are borrowed rather than rendered now** — a text parameter's octets
  are the string's own — which is what leaves `key`, `tag`, `script` and `payload` alive for the
  SQLite arm to bind.
- **The worker is the half that is left.** `crates/nvs-cli/src/worker.rs`'s `Wire` has no SQLite
  arm, so `open` still refuses that driver with a warning and a job pushed onto a SQLite queue is
  enqueued and never claimed. `crates/nvs-stdlib/src/queue.rs`'s gap 4 says exactly that.
- The rustdoc gate that had been red since session 0005 is green: `DELETE_SQLITE`'s doc linked
  `Self::then` from a constant.
- Nothing is blocked, no design call is waiting on the user, and no `[context]` field was missing
  from this session's pack.

## Next group

**Stage 5: the worker's third arm** — one file set: `crates/nvs-cli/src/worker.rs`. The goal's own
§ *Stage 5* item 3, split at the seams the file already has.

- [ ] **`Wire` gains a SQLite arm and `open` stops refusing it** — `crates/nvs-cli/src/worker.rs:932`,
      `crates/nvs-cli/src/worker.rs:866` and the refusal at `crates/nvs-cli/src/worker.rs:909`, under
      `rule:core-classes/db-drivers-are-an-enum`. `nvs_db::sqlite::open` takes a
      `nvs_db::SqliteTarget::resolve`d block, which is `crates/nvs-stdlib/tests/queue_sqlite.rs:71`'s
      shape. The SQL Server arm stays spelled and keeps its sentence.
- [ ] **`Dialect` gains its third arm, and `roster` and `claim` branch on it** —
      `crates/nvs-cli/src/worker.rs:959`, `crates/nvs-cli/src/worker.rs:243` and
      `crates/nvs-cli/src/worker.rs:334`, under `rule:concurrency/claiming-is-one-statement`.
      `CLAIM_SQLITE` is a `Split` inside the immediate transaction, and
      `crates/nvs-stdlib/tests/queue_sqlite.rs:156`'s `claim` is that pair in the order the worker
      runs it; the worker owns its connection, so `depth()` is zero there.
- [ ] **The reporting half takes the arm** — `crates/nvs-cli/src/worker.rs:668`,
      `crates/nvs-cli/src/worker.rs:689` and `crates/nvs-cli/src/worker.rs:715`, under
      `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`. `DEAD_LETTER_SQLITE` is a
      `Split` and `SUCCEEDED_SQLITE`/`RETRY_SQLITE` are single texts.

## Backlog

- Stage 6's cases, the `tools/db-matrix.py` SQLite leg and `crates/nvs-stdlib/tests/queue.rs:313`'s
  own two-armed `Dialect` — `docs/agent/loop-goal.md` § *Stage 6*.
- Nothing covers `sqlite_counted`, `push_in_sqlite` or `sqlite_opened` yet: an in-module
  `#[cfg(test)]` case can, since `nvs_db::sqlite::open` answers a real connection — the playbook's
  bullet on that is under *Writing a test case*.
- `examples/queue-sqlite.nvs`, which the driver's acceptance check names —
  `docs/agent/loop-goal.md` § *Stage 6* item 4.
