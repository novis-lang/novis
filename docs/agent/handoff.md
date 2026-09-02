# Handoff

## State

**ADR 0067 § 4's `query` answers on a MySQL connection.** `queried_rows`
(`crates/nvs-stdlib/src/db.rs:3650`) branches on the filed connection rather than demanding a
`PgConn`: `postgres_rows` is the old body moved whole, `mysql_rows`
(`crates/nvs-stdlib/src/db.rs:3781`) drains a binary result set through § 9's `mysql_column_value`
and `mysql_described_columns`, and all three `#[expect(dead_code)]`s over that decode are gone. The
two halves are deliberately not shared — `mysql_rows`' own doc says what the drivers agree on and
what they do not.

**A statement is rewritten and bound in its connection's own dialect.** `rendering_for`
(`crates/nvs-stdlib/src/db.rs:3484`) pairs § 5's `nvs_db::Dialect` with § 9's encoder in one place,
`rendering_of` reads the driver off the connection through the new `filed_connection`, and
`statement_in` is the old `statement_of` body with that pair passed in — so `executeMany` asks once
per batch rather than once per set. MariaDB renders as MySQL does; SQL Server and SQLite throw
gap 2's message *before* the rewrite instead of after the bind.

**Still PostgreSQL-only, and this is the module's known gap 2 in full**: `execute`, `executeMany`
and `transaction` go through `postgres_of` (`crates/nvs-stdlib/src/db.rs:3568`), and a MySQL `query`
files no § 11 event because a `MySqlRows` carries no span where a `PgRows` opens one.

**The MySQL read path has not met a server.** `verify.py` has no matrix leg, so what holds it is the
type checker plus two new unit tests over `rendering_for`; `tools/db-matrix.py` and
`crates/nvs-db/src/matrix.rs` are the leg that would, and that is the third item below.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for
a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own
ADR, not a slice. Untouched this session.

**`orient.py` gaps:** `[context] adrs` wants ADR 0067 § 2, § 4, § 5, § 11 and § 18. § 4 and § 5
specified both of this session's slices and neither was printed; § 11 specifies the next item.

## Next group

**§ 11 and § 4's two writes, one file set: `crates/nvs-stdlib/src/db.rs`, against
`crates/nvs-db/src/mysql.rs` and `crates/nvs-db/src/span.rs`. The span first — `execute` will want
to file the same event, and doing it after means writing that arm twice.**

- [ ] **§ 11's `query` event over a MySQL statement** — ADR 0067 § 11.
      `crates/nvs-stdlib/src/db.rs:3650` is where the MySQL arm hands back `None` where the
      PostgreSQL one hands `QueryWatch::taken`'s pair, and `crates/nvs-stdlib/src/db.rs:3781` is the
      drain that would open the span. `crates/nvs-db/src/mysql.rs:2174` (`MySqlRows`) carries no
      span field, `crates/nvs-db/src/mysql.rs:1564` (`start_statement`) is where `PgRows`' opens,
      and `crates/nvs-db/src/span.rs:86` is `QuerySpan` itself.
- [ ] **`execute` and `executeMany` over a MySQL connection** — ADR 0067 § 4.
      `crates/nvs-stdlib/src/db.rs:4339` and `crates/nvs-stdlib/src/db.rs:4412` both still reach
      `crates/nvs-stdlib/src/db.rs:3568` (`postgres_of`); § 4's two counts are
      `nvs_db::MySqlRows::affected` and `::last_id` (`crates/nvs-db/src/mysql.rs:2174`), and MySQL's
      `lastId` comes off the status packet with no `RETURNING` to ask for. `executeMany`'s span is
      still `QuerySpan::opened(nvs_db::Driver::Postgres, …)` at that second anchor.
- [ ] **The MySQL read path against a live server** — `crates/nvs-db/src/matrix.rs:1` names the
      `NVS_DB_MATRIX_*` fields and `tools/db-matrix.py` sets them; `examples/db.nvs` is the program
      shape a `[db.<name>] driver = "mysql"` block would run.

## Backlog

- `Core\Db::open` — `nvs_stdlib::db` known gap 1, and the shape-parameter registry type it waits on
  wants its own ADR.
- `transaction` over MySQL — ADR 0067 § 7, after `execute` lands.
- MariaDB has no connect path at all — ADR 0067 § 2; it binds and then has nowhere to send.
- `crates/nvs-stdlib/src/queue.rs:1295`'s own `postgres_of` stays PostgreSQL's — ADR 0084 § 2.
