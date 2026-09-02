# Handoff

## State

**§ 1's statement cache is on a MySQL connection.** `crates/nvs-db/src/sql.rs`'s `StatementCache`
is now generic in its *handle* — `StatementCache<H = String>` — because the two protocols disagree
about who names a statement: PostgreSQL mints the name, and MySQL is handed an id by
`COM_STMT_PREPARE`. Everything above the handle is shared and § 1 states it once. PostgreSQL's
name-minting `prepare` → `Prepared` stays on `impl StatementCache<String>`; MySQL drives the same
entries through `lookup`/`make_room`/`commit`, since the id it records does not exist until the
prepare has landed. The type's own doc owns that split.

**`capacity_for`/`DEFAULT_CAPACITY` are gone from the type**, as the free `statement_cache_for` and
`DEFAULT_STATEMENT_CACHE` beside `time_zone_for` — neither mentions the handle, and a defaulted type
parameter cannot be inferred at an unqualified path (playbook, *Writing Novis itself*). Every caller
and doc link in `pg.rs`, `mysql.rs` and `nvs-config` moved with them.

**On the MySQL side**: `MySqlTarget::statement_cache` (same reader as PostgreSQL's),
`MySqlConn::cache`, `cached_statement` in front of `prepare`, and `close_statement` — `COM_STMT_CLOSE`,
the one command this driver writes and reads no answer to. An eviction is closed *before* the prepare
that needed the room, so the server never holds more than `statement_cache` statements. `reset_session`
now takes the cache and clears it rather than leaving that to its caller: § 13's `COM_RESET_CONNECTION`
drops prepared statements, which is the asymmetry with PostgreSQL that § 13 calls the protocol's.

**Not wired above this crate**: `Core\Db::connect` still opens PostgreSQL only
(`crates/nvs-stdlib/src/db.rs:2529` is the only `PgTarget::resolve` call site), so a `driver = "mysql"`
block resolves in `nvs-db` and reaches no opener.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, which is blocked on a registry
type for a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants
its own ADR, not a slice. This session did not touch it.

**`orient.py` gaps:** `[context] adrs` wants ADR 0067 § 2 (the `[db.<name>]` block as a discriminated
union) and § 18 (`connect`), which the next two slices are written against; § 1, § 9 and § 13 were
printed and were enough for this one.

## Next group

**`Core\Db` opens and reads a MySQL connection — one file set: `crates/nvs-stdlib/src/db.rs`, with
`crates/nvs-db/src/conn.rs` for the enum arms. `crates/nvs-stdlib/src/db.rs:2529` (the PostgreSQL
open, which slice 1 branches) and `crates/nvs-stdlib/src/db.rs:3773` (`column_value`, which slice 2
extends) are what both copy.**

- [ ] **`Core\Db::connect` opens a MySQL block** — ADR 0067 § 2 and § 18.
      `crates/nvs-stdlib/src/db.rs:2529` resolves a `PgTarget` unconditionally; branch on the block's
      `driver` so a `mysql` block reaches `nvs_db::MySqlTarget::resolve` and `MySqlConn::connect`,
      and keep the memoized, request-held connection `crates/nvs-stdlib/src/db.rs:2399`
      (`nvs_core_db_connect`) already hands back. `crates/nvs-db/src/conn.rs:646` (`MySqlConn`) is the
      arm of the `Connection` enum it lands in.
- [ ] **A MySQL row hydrates a Novis row** — ADR 0067 § 9.
      `crates/nvs-stdlib/src/db.rs:3773` (`column_value`) builds a `Value` from `PgScalar`; give it
      the `MySqlScalar` half, whose three structured rows (`MySqlDate`/`MySqlTime` and the datetime)
      mirror `PgDate`/`PgTime` so `date_at`/`time_of_day_at`/`datetime_at` take them unchanged.
- [ ] **`Core\Db::open`'s shape parameter** — the standing acceptance failure, and an ADR rather than
      a slice: `crates/nvs-stdlib/src/db.rs:66` (known gap 1) wants a registry type for a *shape*
      parameter before `a_db_open_target_in_a_denied_range_fails` can run.

## Backlog

- MariaDB is its own driver and has none of the above — `docs/plan/m8.md`.
- SQL Server's TDS 7.4 is the largest piece of new wire code left — `crates/nvs-db/src/conn.rs`.
- SQLite is the audited C dependency and has no wire at all — ADR 0051 § 4.
- The five-driver CI container matrix beyond PostgreSQL and MySQL — `crates/nvs-db/src/matrix.rs`.
