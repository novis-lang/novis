# Handoff

## State

**MySQL decodes a row to Novis values and resolves its own config block.**
`crates/nvs-db/src/mysql.rs` now carries § 9's value half: `scalar(&Column, &MyValue)` answers a
`MySqlScalar` and `decode` mints the `Value`, `PgColumn::scalar`/`PgScalar`'s opposite numbers.
`MySqlScalar` has **three** structured rows, not `PgScalar`'s five — no `Instant` (MySQL has no
`TIMESTAMPTZ`) and no `Uuid` (§ 9 sends MySQL's `BINARY(16)` to `bytes`; the backend with the type
is MariaDB, which is its own driver) — and no array variant. `MySqlDate`/`MySqlTime` mirror
`PgDate`/`PgTime` so `nvs-stdlib`'s `date_at`/`time_of_day_at`/`datetime_at` take them unchanged.

**Two decisions the enum's own docs own**: a binary-charset `ColumnType::Other` column (`BIT(n>1)`,
`GEOMETRY`) decodes as `bytes` where § 9 words that row as `tainted string`, because § 9 was
written against PostgreSQL's text rendering and these octets are not text in any encoding; and a
`FLOAT` is widened through its shortest decimal rather than by `f64::from`, so the same column on
PostgreSQL and MySQL reads back as the same Novis `float`.

**`BlockError` now lives in `crates/nvs-db/src/conn.rs`**, which is the move its own doc named:
`MySqlTarget::resolve` shares it, `OtherDriver`/`Missing`/`Unusable` gained an `expected: Driver`,
and `Driver::display_name` is what puts the right backend in the sentence — every existing message
is byte-identical for PostgreSQL, so `tests/conformance/core/db-connect-refuses-a-block-it-cannot-read-by-the-field.nvst`
is untouched. `written_value` moved with it.

**Not wired above this crate yet**: `Core\Db::connect` still opens PostgreSQL only, so a
`driver = "mysql"` block resolves in `nvs-db` and reaches no opener in `nvs-stdlib`.

**No statement cache**, so every MySQL statement still costs § 1's two round trips.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, which is blocked on a registry
type for a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that
wants its own ADR, not a slice. This session did not touch it.

**`orient.py` gaps:** `[context] adrs` wants ADR 0067 § 2 (the `[db.<name>]` block as a
discriminated union) — the resolve slice was written against `pg.rs`'s code rather than the section
— and § 4/§ 5 for the cache below. § 1, § 9 and § 13 were printed and were enough.

## Next group

**MySQL's statement cache and its Novis-facing half — one file set:
`crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/sql.rs` and `crates/nvs-stdlib/src/db.rs`.
`crates/nvs-db/src/pg.rs:2549` (`start_statement`, the cache's shape) and
`crates/nvs-stdlib/src/db.rs:3773` (`column_value`) are what the first two copy.**

- [ ] **§ 1's statement cache on a MySQL connection** — ADR 0067 § 1 and § 13.
      `crates/nvs-db/src/mysql.rs:1452` (`start_statement`) prepares every time;
      give `MySqlTarget` a `statement_cache` field like `crates/nvs-db/src/pg.rs:202`'s,
      resolved through `StatementCache::capacity_for` in `crates/nvs-db/src/mysql.rs:315`
      (`MySqlTarget::resolve`), and note § 13's asymmetry: `COM_RESET_CONNECTION` drops
      prepared statements, so the reset invalidates the cache where PostgreSQL's does not.
- [ ] **`Core\Db::connect` opens a MySQL block** — ADR 0067 § 2 and § 18.
      `crates/nvs-stdlib/src/db.rs:2353` picks `nvs_db::pg::DEFAULT_PORT` and resolves a
      `PgTarget` unconditionally; branch on `Driver::from_config_name` so a `driver = "mysql"`
      block reaches `nvs_db::mysql::DEFAULT_PORT` and `MySqlConn::connect`.
- [ ] **A MySQL row hydrates a Novis row** — ADR 0067 § 9.
      `crates/nvs-stdlib/src/db.rs:3773` (`column_value`) takes a `nvs_db::PgScalar`;
      it needs the `MySqlScalar` arm for `Date`/`Time`/`DateTime` beside it, over
      `crates/nvs-db/src/mysql.rs:1806` (`scalar`).

## Backlog

- `Core\Db::open` waits on a registry type for a shape parameter — `nvs_stdlib::db` known gap 1.
- MySQL `SET` reads as its comma-joined text, not § 9's `array<string>` — `conn.rs`'s `ColumnType`.
- MariaDB, SQL Server and SQLite have no wire code at all — `crates/nvs-db/src/conn.rs`.
- `executeMany` on MySQL wants `COM_STMT_BULK_EXECUTE`'s absence priced — ADR 0067 § 4.
