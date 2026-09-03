# Handoff

## State

**M8 goal 5, stage 8. SQL Server is whole against a real server** — §§ 1, 3, 4, 7, 9 and 13, with
`python tools/db-matrix.py --driver mssql` green. § 7 works there because the driver now reads the
transaction descriptor: `EnvChange::Transaction` (`crates/nvs-db/src/tds.rs:1801`), kept on
`Wire::descriptor` (`crates/nvs-db/src/tds.rs:747`) because both the token reader and every request
builder already hold the wire, and written into the `ALL_HEADERS` of every RPC and batch. Without it
the server refused the first statement after a `BEGIN` with code 3989.

**§ 13's reset had a real hole, and only a real server could say so.**
`sp_reset_connection` does **not** put the isolation level back: a session that ran
`transaction({isolation: Serializable})` was still `SERIALIZABLE` after the reset, so the next request
off the pool inherited it. `reset_session` now pays the restore it already owed, and that function's
doc comment is the fact's one home. ADR 0067 § 13 needs no amendment — it states the reset as a
property and names the procedure only as the means.

**gap 2 is SQLite, and it is the only driver left.** `SqliteConn` (`crates/nvs-db/src/conn.rs:823`) is
a stub carrying `state` and nothing else; there is no `sqlite.rs`, and `rusqlite` is in the workspace
manifest but not yet a dependency of `nvs-db`. It is the one audited C dependency (ADR 0051 § 4), so
nothing about it is a decision to re-take.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`,
`nvs_types::intrinsics`' known gap 6, and is not this goal's to close.

`orient.py`'s map still prints `nvs-stdlib`'s `json`, `registry` and `time` but not `db`; the goal's
`[context] modules` needs `nvs-stdlib/src/db.rs`.

## Next group

**One file set: a new `crates/nvs-db/src/sqlite.rs` with `crates/nvs-db/src/conn.rs`**, plus the two
lines that admit it (`crates/nvs-db/src/lib.rs:174`, `crates/nvs-db/Cargo.toml:37`). SQLite is the
last driver, and its shape is the opposite of the other four's: no wire, no codec, and every call on
`nvs-host`'s blocking pool.

- [ ] **SQLite opens from its config block and runs § 4's statements**, on the blocking pool rather
      than a readiness registration (0067 §§ 1, 3, 4). `crates/nvs-db/src/conn.rs:823` (`SqliteConn`,
      which gains the handle beside `state`), `crates/nvs-db/src/conn.rs:846` (`Connection`'s `Sqlite`
      arm), `crates/nvs-db/src/lib.rs:174` (the re-export list a `sqlite` module joins),
      `crates/nvs-db/src/sql.rs:72` (`Dialect`, for the marker § 5 rewrites to).
- [ ] **§ 7's nesting and § 13's reset**, which here is rolling back an open transaction and nothing
      else — a file handle has no session state to leak (0067 §§ 7, 13).
      `crates/nvs-db/src/conn.rs:823`, `crates/nvs-db/src/conn.rs:846`.
- [ ] **§ 9's map off the *declared* column type**, throwing on a value that does not parse — SQLite
      has no date or time types (0067 § 9). `crates/nvs-db/src/sql.rs:72`,
      `crates/nvs-db/src/matrix.rs:76` (`Location::Path`, which is how the matrix's sqlite leg names a
      scratch file).

## Backlog

- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` — `nvs_types::intrinsics`' known gap 6.
- `[context] modules` in `docs/agent/loop-goal.toml` is missing `nvs-stdlib/src/db.rs`.
- `Core\Db` over SQLite in `nvs_stdlib::db` once the driver lands — `docs/adr/0067-core-db.md` § 2.
- `examples/` has no SQLite block; `docs/plan/m8.md` owns the five-driver matrix claim.
