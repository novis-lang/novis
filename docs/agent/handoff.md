# Handoff

## State

**Goal `m8-db-queue`, stages 0 and 2 are done.** `docs/decisions/0187.md` is on disk and accepted: it
decides the parked read per driver, what `serverVersion` answers, null-distinct unique keys on all
five backends, and where a job's earlier errors live. Its three `designed` rules exist and
`python tools/rules.py --render` has run, so stage 2's three acceptance checks pass.

**It modifies five fragments, not the four the goal's table names.**
`rule:core-classes/schema-vocabulary-is-closed` is the fifth, because the v1 exclusion of a partial
index lives there rather than in `rule:core-classes/schema-plan` — which is what
`rule:core-classes/queue-storage-is-a-table` had been citing for it, now corrected. `schema-plan` is
still modified, for the grade the filtered index carries.

No code is touched yet. Stage 1 is goal `m7-server-surface`'s carried floor and is the driver's to
run. `python tools/records.py --check` was red before this session on `docs/decisions/0186.md` alone —
two rules whose `because` named it were missing from its `changes:` block — and is now clean.

## Next group

**Stage 3: the keystone — a parked read on the MySQL wire** — one file set:
`crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/conn.rs`, `crates/nvs-db/src/maria.rs` and
`crates/nvs-stdlib/src/db/stream.rs`. MySQL and MariaDB share one row loop, so one parked state covers
both. `rule:core-classes/a-stream-parks-its-read-on-the-connection` is what it must satisfy, and
`docs/decisions/0187.md` § 1 is the reasoning.

- [ ] **Split the MySQL read state from the borrow.** `crates/nvs-db/src/mysql.rs:3087` `MySqlRows`
      holds the wire and the read state together; park the read half on `crates/nvs-db/src/conn.rs:832`
      `MySqlConn` the way `crates/nvs-db/src/pg.rs:2555` `PgCursor` is parked on `PgConn`, with one row
      reader serving both paths as `crates/nvs-db/src/pg.rs:2773` `next_row_of` does.
      `crates/nvs-db/src/conn.rs:439` `State::Streaming` is the refusal
      (`rule:core-classes/db-connection-busy-state`). Prove it against a scripted server, the way
      `pg.rs`'s generic `Wire` is proved.
- [ ] **`stream_step` grows the arm.** `crates/nvs-stdlib/src/db/stream.rs:133` `stream_step` gains a
      `Connection::MySql`/`MariaDb` arm beside the PostgreSQL one, and
      `crates/nvs-stdlib/src/db/stream.rs:294` `unstreamed` stops naming PostgreSQL as the only driver
      that streams.
- [ ] **The real-server half joins the matrix.** New `crates/nvs-stdlib/tests/db_stream.rs`, gated on
      `NVS_DB_MATRIX_DRIVER` the way `crates/nvs-stdlib/tests/queue.rs` is, listed in
      `tools/db-matrix.py:115` `SUITES` — the matrix runs only `nvs-db` and `--test queue` today, so
      nothing checks a stdlib stream against a server.

## Backlog

- Stage 4 (SQL Server's token walk and SQLite's pinned thread) is the group after this one —
  `docs/agent/loop-goal.md` § *Stage 4*.
- The `errors` entry's message cap that `docs/decisions/0187.md` § 4 requires has no constant yet;
  stage 7 picks the number beside `crates/nvs-stdlib/src/queue.rs:882` `dead_errors`.
- Every `— owner: m8-db-queue` tag is still on the tree; stage 10 is what removes them.
- `docs/agent/loop-goal.md`'s stage anchors run one to three lines off throughout — re-grep rather
  than trusting them.
