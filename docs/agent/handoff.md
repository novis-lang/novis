# Handoff

## State

**`Core\Db::connect` opens two drivers.** `crates/nvs-stdlib/src/db.rs`'s `open_named` reads the
block's `driver` (ADR 0067 § 2) and branches: a `mysql` block reaches `MySqlTarget::resolve`,
port 3306 and `MySqlConn::connect`, everything else — including a block writing no `driver` and one
naming a backend that has no opener — goes to PostgreSQL, whose `resolve` is where those refusals
are worded and names what was written. `address_of` takes the driver's default port as a parameter
now; nothing else about the memo, the capability check or § 13's ticket moved.

**A MySQL connection is pooled too.** `warm_connection` answers a whole `nvs_db::Connection` rather
than a `PgConn` and resets either driver, so the two variants that have a reset behind them rejoin
the pool and the other three are still dropped there. The two resets differ in what survives —
PostgreSQL keeps § 1's cache, `COM_RESET_CONNECTION` drops it — and each driver's own `reset` is
where that is met, not this function.

**Nothing runs a statement on it yet.** `postgres_of` (`crates/nvs-stdlib/src/db.rs:3417`) is what
every member goes through, so a MySQL connection opens, memoizes, pools and resets, and then refuses
`query`/`execute`/`transaction` by name. That is the module's known gap 2, which now states the
split; gap 3 was stale about the pool and states the driver bound instead.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for
a shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its
own ADR, not a slice. Untouched this session.

**Only slice 1 of the last group landed.** Slice 2 is not one slice: the read path is `PgRows`-shaped
in four separate places, listed below.

**`orient.py` gaps:** `[context] adrs` still wants ADR 0067 § 2 (the `[db.<name>]` block as a
discriminated union) and § 18 (`connect`) — the item named both and neither was printed; § 1, § 9
and § 13 were. The next group wants § 4 as well.

## Next group

**The MySQL read path, one file set: `crates/nvs-stdlib/src/db.rs`, against `crates/nvs-db/src/mysql.rs`
for the shapes it decodes. MySQL's row surface is complete in `nvs-db` and parallel to PostgreSQL's
without being the same shape — `MySqlRow::value(index)` hands back a `&MyValue` where a `PgRow` lends
bytes a `PgColumn` types, so the drain is a second loop and not a generic one.**

- [ ] **A MySQL column becomes a Novis value** — ADR 0067 § 9. `crates/nvs-stdlib/src/db.rs:3830`
      (`column_value`) is `PgScalar`-shaped; write its twin over `nvs_db::MySqlScalar`
      (`crates/nvs-db/src/mysql.rs:1800`, minted by the free `scalar` at
      `crates/nvs-db/src/mysql.rs:1919`), and `crates/nvs-stdlib/src/db.rs:3744`
      (`described_columns`) takes `&[nvs_db::PgColumn]` where MySQL lends `mysql_common::Column`
      (`crates/nvs-db/src/mysql.rs:2116`) typed through `column_type`
      (`crates/nvs-db/src/mysql.rs:2123`).
- [ ] **A MySQL connection answers `query`** — ADR 0067 § 4. `crates/nvs-stdlib/src/db.rs:3417`
      (`postgres_of`) is the one accessor and refuses every other driver by name; add its MySQL
      twin, and branch the drain at `crates/nvs-stdlib/src/db.rs:3510` (`queried_rows`) over
      `MySqlRows::next_row` (`crates/nvs-db/src/mysql.rs:2174`).
- [ ] **§ 11's `query` event over a MySQL statement** — `crates/nvs-stdlib/src/db.rs:3727`
      (`name_span`) takes a `&mut nvs_db::PgRows`, so a MySQL statement files no span at all until
      it has one; `QueryWatch` above it is already driver-neutral.

## Backlog

- `Core\Db::open`'s shape parameter — `nvs_stdlib::db` known gap 1, and the standing acceptance
  failure. Wants an ADR, not a slice.
- `execute`, `executeMany` and `transaction` over MySQL — the same `postgres_of` wall, after the
  group above.
- `crates/nvs-stdlib/src/queue.rs:1295` has its own `postgres_of`; ADR 0084's queue stays
  PostgreSQL-only until § 2's schema has a MySQL spelling.
- MariaDB, SQL Server and SQLite have no opener — `nvs_db::Connection`'s other three variants.
