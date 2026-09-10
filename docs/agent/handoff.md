# Handoff

## State

**Goal `sqlite-queue`, stage 5 is landed whole — both halves.** Every `Core\Queue` member sends over
a SQLite connection (stage 5's stdlib half, already on disk) and the worker now claims and reports
over one: `Wire` at `crates/nvs-cli/src/worker.rs:1234` has a fourth arm, `Dialect` at
`crates/nvs-cli/src/worker.rs:1271` a third, and `roster`, `claim` and `report` each branch to it.
The goal's one ADR slot stays taken by [0170](../decisions/0170.md).

- **`open` refuses SQL Server alone**, still spelled rather than left to a `_`. A SQLite block goes
  through `sqlite_wire`, which resolves a path where every other arm resolves an address, so it is
  its own function and not an `open_as!` arm — no host, no port, no `CONNECT_DEADLINE`.
- **The claim is `CLAIM_SQLITE`'s pair inside `begin_immediate`**, which is the whole mutual
  exclusion on this backend (`rule:concurrency/claiming-is-one-statement`), and the write-backs are
  the framed drivers' own texts under the SQLite aliases.
- **Each arm of `roster` and `claim` builds what it sends.** An instant is the octets of its decimal
  text to a wire driver and the integer itself to SQLite, so one shared binding would have been
  built for an arm that cannot use it.
- **`nvs queue migrate` already converges a SQLite block** — `crates/nvs-cli/src/queue.rs:69` says
  there is no third refusal, because the schema is one value emitted in every dialect. Stage 6's
  fixture needs no new command.
- Nothing is blocked, no design call is waiting on the user, and no `[context]` field was missing
  from this session's pack.

## Next group

**Stage 6: the fixture and the two suites** — the goal's own § *Stage 6*, whose items 1, 2 and 4
are below. They are three small files rather than one set; take them in this order, because the
first is the acceptance check the driver has been failing since session 0006.

- [ ] **`examples/queue-sqlite.nvs` exists and runs with no server** —
      `docs/agent/loop-goal.toml:11` is where the frozen `files` list names it, and
      `examples/queue-purge.nvs:1` is the shape beside it, under
      `rule:core-classes/queue-storage-is-a-table`. `examples/queue.nvs` belongs to goal `database`
      and its lines are frozen, so this is its own file rather than an edit to that one.
- [ ] **`crates/nvs-stdlib/tests/queue.rs`'s own `Dialect` gains its third arm** —
      `crates/nvs-stdlib/tests/queue.rs:314`, under `rule:core-classes/db-drivers-are-an-enum`. Its
      doc at `crates/nvs-stdlib/tests/queue.rs:310` still says a third arm would be a driver with
      nothing to send, and that sentence goes with the arm.
- [ ] **`tools/db-matrix.py`'s SQLite leg runs the queue suites** — `tools/db-matrix.py:165` is the
      leg ("a scratch file, no container"), and the suite list it runs is what this extends, under
      `rule:core-classes/db-one-api`'s own *Verification* section.

## Backlog

- The conformance cases for the members, beside goal `queue-purge`'s — goal § *Stage 6* item 3;
  no differential case, because PHP has no queue.
- A sentence where an **operator** reads `[queue] workers` saying where the number stops buying
  anything on SQLite — `nvs.toml`'s `[queue]` block; the engine-side half is now
  `crates/nvs-cli/src/worker.rs`'s module doc § *What `[queue] workers` buys on SQLite*.
- A dead-lettered row still carries the last attempt's error alone — `crates/nvs-cli/src/worker.rs`
  § *Known gap*, owned by M8.
