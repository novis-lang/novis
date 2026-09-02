# Handoff

## State

**§ 9's MySQL decode is on the stdlib side, one slice ahead of its caller.**
`crates/nvs-stdlib/src/db.rs` has `mysql_column_value` and `mysql_described_columns` — the twins of
`column_value` and `described_columns` over `nvs_db::MySqlScalar` and a MySQL result set's own
column definitions. Both carry `#[expect(dead_code)]`, which the branch that calls them takes off
by itself. Three structured rows and not five: MySQL has no `UUID` column type and no array type,
so § 9's recursion has nothing to recur over.

**A MySQL parameter has its own encoder.** `nvs_db::mysql::encode`
(`crates/nvs-db/src/mysql.rs:1915`) renders a bound value as MySQL reads a `VAR_STRING` parameter.
`nvs_db::encode` is PostgreSQL's and is not reusable — the playbook bullet has the three rows that
differ and why each is a wrong row rather than an error.

**Nothing runs a MySQL statement yet.** `postgres_of` (`crates/nvs-stdlib/src/db.rs:3423`) is still
the one route past the handshake, and `statement_of` rewrites every statement in
`nvs_db::Dialect::PostgreSql`. That is the module's known gap 2, which now names both halves that
are on disk ahead of it.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for
a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its
own ADR, not a slice. Untouched this session.

**`orient.py` gaps:** `[context] adrs` still wants ADR 0067 § 2 and § 18, and now § 4 and § 5 —
the group's remaining items are specified by § 4 (`query`) and § 5 (the rewriter and its dialect),
and neither was printed. § 1, § 9 and § 13 were.

## Next group

**MySQL's `query`, one file set: `crates/nvs-stdlib/src/db.rs`, against `crates/nvs-db/src/mysql.rs`
for the encoder and the row surface it drains. Take them in this order — the dialect and the
encoder are what the drain sends, so a drain written first has nothing correct to send.**

- [ ] **A statement is rewritten and bound in its connection's own dialect** — ADR 0067 § 5.
      `crates/nvs-stdlib/src/db.rs:3076` (`statement_of`) hardcodes `nvs_db::Dialect::PostgreSql`
      and `nvs_db::encode`; both follow the driver, which is `nvs_db::Dialect::of`
      (`crates/nvs-db/src/sql.rs:86`) over the connection reached at
      `crates/nvs-stdlib/src/db.rs:3423`. `batch_of` binds the same two ways
      (`crates/nvs-stdlib/src/db.rs:3174`), and `crates/nvs-stdlib/src/queue.rs:1295` has its own
      `postgres_of` that stays PostgreSQL's.
- [ ] **A MySQL connection answers `query`** — ADR 0067 § 4. `crates/nvs-stdlib/src/db.rs:3516`
      (`queried_rows`) drains a `PgRows`; MySQL's drain is `next_row` plus
      `nvs_db::mysql::scalar` over `answered.columns()` — the row is owned, so the columns are
      borrowed again after each `next_row` — into `crates/nvs-stdlib/src/db.rs:3963`
      (`mysql_column_value`) and `crates/nvs-stdlib/src/db.rs:3790`
      (`mysql_described_columns`), both of which lose their `#[expect(dead_code)]` here.
      `postgres_of` (`crates/nvs-stdlib/src/db.rs:3423`) splits into a driver read and one
      accessor per driver, since `watch.file(ctx, …)` needs the connection's borrow to have ended.
- [ ] **§ 11's `query` event over a MySQL statement** — ADR 0067 § 11.
      `crates/nvs-db/src/mysql.rs:2178` (`MySqlRows`) carries no `QuerySpan` at all, and
      `crates/nvs-stdlib/src/db.rs:3733` (`name_span`) takes a `PgRows`. Until this lands a MySQL
      statement files no event, which is the one thing the drain above leaves undone.

## Backlog

- `Core\Db::open` waits on a registry type for a shape parameter — `nvs_stdlib::db` known gap 1,
  and the driver's stage-2 check.
- `execute`, `executeMany` and `transaction` are still PostgreSQL-only — `nvs_stdlib::db` gap 2.
- § 9's MySQL `SET` row (`array<string>`) decodes on neither side — ADR 0067 § 9.
- The three drivers with no connect path — ADR 0067 § 2, `docs/plan/m8.md`.
- ADR 0067 § 4's `stream` is unwritten on both drivers.
