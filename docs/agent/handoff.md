# Handoff

## State

**MySQL reads a result set to its end.** `crates/nvs-db/src/mysql.rs` has ADR 0067 § 9's row half:
`start_statement` now answers a `MySqlRows`, whose `next_row` takes one binary row packet at a time
until the `CLIENT_DEPRECATE_EOF` terminator returns the connection to `State::Idle`. `MySqlRows`
mirrors `crate::pg::PgRows` — it holds the wire and the busy state, so § 4's one-statement-at-a-time
rule is a borrow rather than a check, and its `Drop` drains an abandoned result set so § 13's pool
can still take the connection. A second result set (`SERVER_MORE_RESULTS_EXISTS`) is refused and
poisons, because § 4's surface has nowhere to put one.

**§ 9's type map is `column_type(&Column)`**, the classification half — `PgColumn::column_type`'s
opposite number. What is *not* here is the value half: `MySqlRow::value` hands back
`mysql_common`'s own `Value`, and turning one into a Novis `Value` is the next slice, on the
boundary `PgColumn::decode`/`PgScalar` already occupies for the other driver.

**`MySqlRow` is decoded whole where `PgRow` slices lazily**, and `decode_row`'s doc owns why: a
binary value states no width, so finding column five means decoding the four before it. That
function is also written out rather than delegated to `mysql_common`'s `RowDeserializer`, which
packs every integer into `Value::Int` and so cannot represent § 9's `uint` row at all —
`a_bigint_unsigned_past_i64s_range_is_a_uint_and_not_a_negative_int` is the bound that pins it.

**No statement cache yet**, so every statement still costs § 1's two round trips.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, which is blocked on a registry
type for a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that
wants its own ADR, not a slice. This session did not touch it.

**`orient.py` gaps:** `[context] adrs` wants ADR 0067 § 4 (the busy state's contract) and § 5 (the
placeholder rewriter) for the slices below. § 1, § 9 and § 13 were printed and were enough here.

## Next group

**MySQL's Novis values, its resolve and its cache — one file set:
`crates/nvs-db/src/mysql.rs`, `crates/nvs-db/src/pg.rs` and `crates/nvs-db/src/conn.rs`.
`crates/nvs-db/src/pg.rs:1781` (`PgColumn::scalar`) and `crates/nvs-db/src/pg.rs:2676`
(`start_statement`) are the shapes all three copy.**

- [ ] **A `MySqlRow`'s value becomes the Novis value § 9's table names** — ADR 0067 § 9.
      `crates/nvs-db/src/mysql.rs:1410` (`MySqlRow`) hands back `mysql_common`'s `Value`;
      give `Column` a `decode`/`scalar` pair like `crates/nvs-db/src/pg.rs:1765`'s, with
      `crates/nvs-db/src/pg.rs:1422` (`PgScalar`) as the enum to mirror for `Date`, `Time`,
      `DateTime` and `Uuid` — the five structured rows `nvs-stdlib` is the only crate that can
      allocate.
- [ ] **`MySqlTarget::resolve` off a `[db.<name>]` block** — ADR 0067 § 2, mirroring
      `crate::pg`'s. `crates/nvs-db/src/mysql.rs:246` (`MySqlTarget`) is the struct it fills.
- [ ] **§ 1's statement cache on this connection** — ADR 0067 § 1 and § 13.
      `crates/nvs-db/src/sql.rs:263` (`Prepared`) and `crate::pg`'s `StatementCache` are the shape;
      `crates/nvs-db/src/mysql.rs:1339` (`start_statement`) is where the hit skips `COM_STMT_PREPARE`,
      and § 13's `COM_RESET_CONNECTION` must invalidate it in
      `crates/nvs-db/src/mysql.rs:1163` (`reset_session`).

## Backlog

- `Core\Db::open` needs a registry type for a shape parameter — `nvs_stdlib::db` known gap 1, and it
  wants its own ADR before either blocked acceptance test can be written.
- `close_statement` (`COM_STMT_CLOSE`) has no caller until the cache evicts — `crates/nvs-db/src/mysql.rs`.
- MySQL has no `QuerySpan` yet; ADR 0067 § 11's `query` event is PostgreSQL-only — `crates/nvs-db/src/span.rs`.
- MariaDB is its own driver and has none of this — `docs/adr/0067-core-db.md` § 2.
- The five-driver CI matrix anchors each driver but runs no MySQL row case — `crates/nvs-db/src/matrix.rs`.
