# Handoff

## State

**M8 goal 5, stage 8. SQLite is the fifth driver and it is whole below `nvs-stdlib`** —
`crates/nvs-db/src/sqlite.rs` opens a `[db.<name>]` block off the blocking pool and holds § 4's
statements, § 7's nesting, § 8's kinds, § 9's column map and § 13's reset. Its unit tests are the
real engine: `:memory:` is a path like any other, so every case runs the statements it claims to.

**§ 7's nesting is `crates/nvs-db/src/pg.rs`'s rule reused** — `savepoint_name` and
`open_transaction` are shared, `BEGIN` at depth 0 and `SAVEPOINT nvs_<n>` above it. One accounting
difference is this backend's and is argued at `crates/nvs-db/src/sqlite.rs:660`: a refused outermost
`COMMIT` leaves the depth where it was, because `SQLITE_BUSY` leaves the transaction open and
retryable where PostgreSQL has already rolled it back.

**Two § 7 option calls were decided here and depart from the previous handoff's sketch, which said
to refuse every level but `Serializable`.** Every one of the five isolation levels is *accepted* and
none renders to a command: SQLite is always serializable, so a level asked for is delivered at least
as strongly as asked, which `crate::conn::Isolation`'s own doc makes explicitly not the case § 7 says
to throw over. `read_only` **is** refused at any depth, because SQLite has no read-only transaction
and the only per-session spelling — `PRAGMA query_only` — would be exactly the session state § 13's
"a file handle has no session state to leak" rests on. Both are argued at
`crates/nvs-db/src/sqlite.rs:566`.

**§ 13's reset asks the engine, not the count**: `is_autocommit` decides whether a `ROLLBACK` is
owed, so it also covers a transaction a caller opened in its own § 4 statement text. § 4's
"streaming connections are not resettable" half is held by the borrow checker rather than by a case —
see the new playbook bullet.

**§ 9's map keys off the declared column type** (`SqliteColumn::column_type`,
`crates/nvs-db/src/sqlite.rs:315`): § 9's own names first, SQLite's affinity rules as the fallback.
Never `Uint` (no unsigned storage class) and never `Json` (SQLite has no JSON type, so a declared
`JSON` is `Text`, as on MariaDB). The throw on a value that does not parse is `nvs-stdlib`'s half and
is not written yet.

**Nothing above `nvs-db` reaches any of it.** `nvs-stdlib`'s dispatch still has no `Sqlite` arm, so a
`[db.x] driver = "sqlite"` block is not openable from Novis code. That is the whole of the next
group.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`,
`nvs_types::intrinsics`' known gap 6, and is not this goal's to close.

`orient.py`'s map still prints `nvs-stdlib`'s `json`, `registry` and `time` but not `db`; the goal's
`[context] modules` needs `crates/nvs-stdlib/src/db.rs`, and every item below is inside it.

## Next group

**One file set: `crates/nvs-stdlib/src/db.rs`**, with `crates/nvs-db/src/sqlite.rs` read-only as the
surface being called. Item 1 is what makes the other two reachable, so it goes first.

- [ ] **`nvs-stdlib` admits the `Sqlite` arm**, so § 2's `connect` and `open` reach the driver and
      § 4's `query`/`execute`/`executeMany` run over it (0067 §§ 2, 4).
      `crates/nvs-stdlib/src/db.rs:3223` and `crates/nvs-stdlib/src/db.rs:4357` (the two places
      `Driver::Sqlite` is currently refused or answered with `None`),
      `crates/nvs-db/src/sqlite.rs:488` (`query`, whose owned `Vec<SqliteValue>` is what the binder
      must build — the other four hand over borrowed wire bytes).
- [ ] **§ 9's typed readers over `SqliteValue`**, throwing on a cell the declared column type says
      should parse and does not — a `date` column holding `'not a date'` (0067 §§ 6, 9).
      `crates/nvs-db/src/sqlite.rs:315` (`column_type`, the description this reads),
      `crates/nvs-stdlib/src/db.rs:4357` (where a driver's column becomes `Core\Db\Column::type`).
- [ ] **§ 7's `transaction` over the SQLite arm**, including `{retries: n}` on `SQLITE_BUSY` — § 8
      already normalises it to `Deadlock`, so the retry should work with no new mechanism (0067
      §§ 7, 8). `crates/nvs-stdlib/src/db.rs:4472` (`begin`, where the driver match lives),
      `crates/nvs-db/src/sqlite.rs:590` (`SqliteConn::begin`, and the two options it refuses).

## Backlog

- § 4's `stream` needs a chunk size on SQLite; `crates/nvs-db/src/sqlite.rs`'s module doc owns why.
- § 13's pool has no SQLite entry yet — `crates/nvs-db/src/conn.rs`'s `Connection::reset` dispatch.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`, `nvs_types::intrinsics` known gap 6.
- `[context] modules` in `docs/agent/loop-goal.toml` is missing `nvs-stdlib/src/db.rs`.
