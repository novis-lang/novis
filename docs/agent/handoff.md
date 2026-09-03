# Handoff

## State

**M8 goal 5, stage 8. SQLite is the fifth driver and it opens** — `crates/nvs-db/src/sqlite.rs`
resolves a `[db.<name>]` block, opens the file off the core, and runs § 4's statements with § 8's
kinds on every refusal. It is the one driver whose unit tests are the real engine: `:memory:` is a
path like any other, so `crates/nvs-db/src/sqlite.rs:660` onward exercises the schema, the bind, the
step and every error kind without a server anywhere.

**Its shape is the opposite of the other four's, and the module doc is that fact's one home.** There
is no wire and no codec, so `nvs_host::blocking::run`'s `Send + 'static` bound is what decides the
surface: the handle is an `Arc<Mutex<rusqlite::Connection>>`, parameters arrive owned as
`Vec<SqliteValue>`, and rows come back materialized. § 4's `LogicError` survives that anyway —
`SqliteRows` borrows the connection and holds `State::Streaming` until it drops.

**Two defaults were decided here and are argued at `crates/nvs-db/src/sqlite.rs:359`.**
`PRAGMA foreign_keys = ON`, because § 8 declares `ForeignKeyViolation` a kind every driver
normalises onto and with the pragma off that condition cannot arise at all; and *no* busy timeout,
because `SQLITE_BUSY` is § 8's `Deadlock` and § 7's `retries` is the mechanism written for it.
`rusqlite` is taken with `bundled` — the engine an audit was written against has to be the one the
lockfile pins — and its `libsqlite3-sys` row is in `tools/gen-attribution.py`'s ledger.

**Nothing above `nvs-db` reaches it yet.** `nvs-stdlib`'s dispatch still has no `Sqlite` arm, so a
`[db.x] driver = "sqlite"` block is not openable from Novis code.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`,
`nvs_types::intrinsics`' known gap 6, and is not this goal's to close.

`orient.py`'s map still prints `nvs-stdlib`'s `json`, `registry` and `time` but not `db`; the goal's
`[context] modules` needs `nvs-stdlib/src/db.rs`, and it will be needed by item 3 below.

## Next group

**One file set: `crates/nvs-db/src/sqlite.rs` with `crates/nvs-db/src/conn.rs`**, and item 3 adds
`crates/nvs-stdlib/src/db.rs`. Items 1 and 2 are the rest of goal 5's SQLite item; item 3 is what
makes any of it reachable from Novis.

- [ ] **§ 7's nesting and § 13's reset**, which here is `BEGIN`/`SAVEPOINT` by depth and a rollback
      as the whole reset — a file handle has no session state to leak (0067 §§ 7, 13).
      `crates/nvs-db/src/sqlite.rs:378` (`impl SqliteConn`, where `begin`/`reset` join),
      `crates/nvs-db/src/conn.rs:824` (`SqliteConn`, which gains `depth`),
      `crates/nvs-db/src/tds.rs:5611` (`TdsConn::reset`, the by-value shape a reset takes).
      SQLite is always serializable, so § 7's `Isolation` refuses every level but `Serializable`
      rather than rendering it.
- [ ] **§ 9's map off the *declared* column type**, throwing on a value that does not parse (0067
      § 9). `crates/nvs-db/src/sqlite.rs:255` (`SqliteColumn::declared`, already carried and already
      known to arrive upper-cased), `crates/nvs-db/src/sqlite.rs:187` (`SqliteValue`, the five
      storage classes the map reads from), `crates/nvs-db/src/conn.rs:427` (`ColumnType`, the
      fourteen cases `columns()` answers with).
- [ ] **`nvs-stdlib` admits the `Sqlite` arm**, so `connect` and `open` reach the driver (0067 §§ 2,
      4). `crates/nvs-stdlib/src/db.rs:4225`, `crates/nvs-stdlib/src/db.rs:4357` (the two places
      `Driver::Sqlite` is currently answered with `None`), `crates/nvs-db/src/sqlite.rs:405`
      (`query`, whose owned `Vec<SqliteValue>` is what the binder builds).

## Backlog

- The five-driver matrix's `sqlite: ok` leg asserts nothing driver-specific yet — `tools/db-matrix.py`.
- § 4's `stream` has no SQLite answer; materialized rows make it a chunk size — `docs/adr/0067-core-db.md` § 4.
- `[context] modules` is missing `nvs-stdlib/src/db.rs` — `docs/agent/loop-goal.toml`.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` — `crates/nvs-types/src/intrinsics.rs`.
