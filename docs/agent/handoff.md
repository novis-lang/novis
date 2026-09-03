# Handoff

## State

**MySQL and MariaDB reach `Core\Db` through one body.** `Framed` in
`crates/nvs-stdlib/src/db.rs` is the seam, and its doc owns the reading — MariaDB is its own driver
*above* the framing, not inside it, because `nvs_db::MariaConn`'s `query` delegates into the same
`nvs_db::mysql::start_statement` and hands back the same `MySqlRows`. `connect`, `open`,
`query`/`execute`/`executeMany`, § 7's `transaction` and § 13's pooled reset each have a MariaDB arm
now; known gap 2's roster in that module's doc is three drivers, and
`the_refusal_names_every_driver_that_sends` holds `driverless`'s sentence to the arms rather than to
whoever last edited it.

**The item this session opened on was already on disk.** § 8's `sql` landed in `fe39a987` and
`b4e23338` — `nvs_runtime::SQL_SLOT`, `nvs_hir::errors::SQL_SLOT`, the `OWN_PROPERTIES` row and
`statement_failure`'s write of it — and `docs/agent/loop-goal.toml`'s stage-9 program check already
pins the spelling with `sql=insert into people (id, name) values (?, ?)`. Nothing was owed; the
playbook bullet added this session is the check that would have said so in one call.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. It is not starving the acceptance list either: `.loop/log.md`
reads 194 checks an iteration. Do not rename it, split it, or write a MySQL-only test under it.

## Next group

**One file, `crates/nvs-stdlib/src/queue.rs`.** ADR 0084 § 2's queue is the last PostgreSQL-only
surface in `nvs-stdlib`. `Framed` is the shape to copy and not the code to share: these are the
queue's own statements, and what differs between the drivers here is the SQL rather than the send.

- [ ] **The claim path refuses the two drivers that now open** (0084 § 2).
      `crates/nvs-stdlib/src/queue.rs:1295` is `postgres_of` and
      `crates/nvs-stdlib/src/queue.rs:1315` is its one `let nvs_db::Connection::Postgres(…) else`;
      widen both to the roster `crates/nvs-stdlib/src/db.rs`'s `driverless` now names, and decide
      there whether the queue borrows `Framed` or keeps a refusal of its own.
- [ ] **§ 2's schema is written in one dialect** (0084 § 2).
      `crates/nvs-stdlib/src/queue.rs:192` is the DDL and the claim-order index beside it; `bigint`,
      `text` and the identifier quoting are what a MySQL server reads differently, and § 5's
      rewriter does not touch a `CREATE TABLE`.
- [ ] **§ 4's claim needs a dialect, not a translation** (0084 § 4).
      `crates/nvs-stdlib/src/queue.rs:298` is `CLAIM`, a `with … as` CTE ending in
      `for update skip locked`; ADR 0084 § 4 already states the per-backend spelling, MySQL 8 has
      `SKIP LOCKED` and MariaDB 10.6 does too, so this is a rendering and not a second algorithm.

## Backlog

- SQL Server has no driver at all, so stage 7's `mysql_and_mssql_…` check stays open —
  `docs/agent/loop-goal.toml`, stage 7.
- SQLite is ADR 0051 § 4's one audited C dependency and still has no connection path —
  `crates/nvs-stdlib/src/db.rs` known gap 2.
- `Core\Db::open`'s arm selection — that module's known gap 1.
- Known gaps 4-8 in `crates/nvs-stdlib/src/db.rs`'s module doc, none of them blocking.
- The five-driver CI matrix beyond the three that open — `crates/nvs-db/src/matrix.rs`.
