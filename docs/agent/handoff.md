# Handoff

## State

**Goal 21, stage 5 is the live stage**, and the driver's acceptance run stops there: `native
examples/stream.nvs [5 db tail]` is item 8 and is unwritten, so only 57 of the goal's ~364 checks
are reached. That fixture is the stand-in session 0003 wrote and its own comment says what replaces
it; nothing regressed.

**Item 9 lands three quarters of the way.** `Core\Db\Connection::close`, `driver()` and `isOpen()`
are registered, implemented and asked by three SQLite conformance cases, and
`close_releases_the_connection_and_a_later_member_refuses` is the first of stage 5's six named tests
to go green. `close` is ADR 0067 § 13's release reached early for one connection —
`nvs_runtime::Ctx::close_open_connection` is the single home and the teardown loop skips what it
emptied. A closed entry keeps its key and loses its connection, which is what tells a
`close`-then-use (a `LogicError`, raised once in `crate::db::pool::filed_connection`) from a handle
this crate filled wrongly (a `Fault::fatal`). `serverVersion` is the fourth row and is owed for a
reason that is not this crate's: no driver keeps the server's version string. That is
`crates/nvs-stdlib/src/db/mod.rs`'s gap 5.

**Item 8's triage, which is what the next session most needs.** The driver half is already there:
every backend has a row-at-a-time reader — `crates/nvs-db/src/pg.rs:2456`,
`crates/nvs-db/src/mysql.rs:3006`, `crates/nvs-db/src/sqlite.rs:482`,
`crates/nvs-db/src/tds/rows.rs:177` — and `State::Streaming` is already a connection state. What
blocks `stream` is that `PgRows<'a>` is a **borrow** of the connection, and a Novis object has to
hold the cursor across `advance()` calls: the read state has to move onto the connection itself
before `nvs_stdlib::cursor`'s three-name protocol can carry it. `Core\Db\Rows` cannot be reused as
the answer — it holds every decoded row in one slot by construction.

Nothing is blocked on a decision.

## Next group

**Item 8, in the order that makes each slice landable.** One file set:
`crates/nvs-db/src/pg.rs`, `crates/nvs-db/src/conn.rs`, `crates/nvs-stdlib/src/db/`,
`crates/nvs-stdlib/src/cursor.rs`.

- [ ] **A PostgreSQL statement's read state lives on the connection, not on a borrow of it** —
      `crates/nvs-db/src/pg.rs:2293` is `PgRows`, `crates/nvs-db/src/pg.rs:2456` is `next_row` and
      `crates/nvs-db/src/conn.rs:871` is the enum a `Core\Db\Connection` holds. `PgRows` stays as the
      buffered path's borrow; what is added is the resumable form a held cursor advances, with the
      portal left open and `State::Streaming` the state it sits in. ADR 0067 §§ 4 and 18.
- [ ] **`Core\Db\Connection::stream` answers an `Iterable<Db\Row>` over that cursor** —
      `crates/nvs-stdlib/src/db/registry.rs:228` is `CONNECTION`, `crates/nvs-stdlib/src/cursor.rs:76`
      is the `iterate`/`advance`/`current` shape to copy, and `crates/nvs-stdlib/src/db/mod.rs:433`
      is the `address` roster. § 18's connection-busy refusal is a `LogicError` and belongs beside the
      one `crates/nvs-stdlib/src/db/pool.rs:255` already raises. Tests:
      `stream_answers_rows_without_holding_the_result_set`,
      `a_second_statement_on_a_streaming_connection_is_a_logic_error`.
- [ ] **`examples/stream.nvs` becomes the program its own comment describes** —
      `examples/stream.nvs:32` is the stand-in `echo`. Its three frozen lines are
      `docs/agent/loop-goal.toml:4133`; it needs the compose PostgreSQL, which the plan's *Blocking*
      says is up on this machine.

## Backlog

- `serverVersion`, the fourth § 18 row — `crates/nvs-stdlib/src/db/mod.rs:171` (gap 5) says what each
  driver would have to keep.
- Item 10's `[db.<name>.pool]` bounds and unscoped `pool = false` — same gap 5 block, item 1.
- Item 6, `nvs check` reads the configuration — the previous handoff's group, untaken:
  `crates/nvs-cli/src/main.rs:717`, `crates/nvs-types/src/lib.rs:404`.
- `streamAs`, which is `stream` plus `queryAs`'s hydration — `crates/nvs-stdlib/src/db/row.rs`.
- `orient.py`'s `[context] modules` names `crates/nvs-stdlib/tests/spec_registry_coverage.rs`, which
  matched no module — it is a test file, so the pattern belongs in a different field or nowhere.
