# Handoff

## State

**M8 goal 5, known gap 5 closed.** `crates/nvs-stdlib/src/queue.rs`'s gap list is the one home and
now says it: all four members, `nvs-cli`'s worker and this file's own cases run on either dialect,
and what is left is the two drivers that send no statement at all (`crate::db`'s gap 2).

**`crates/nvs-stdlib/tests/queue.rs` is driver-agnostic and asserts against real MySQL and
MariaDB.** Four types carry it, mirroring `crates/nvs-cli/src/worker.rs`: `Leg` is the driver plus
the server the harness published, `Conn` is the owned connection with one arm per driver,
`Dialect` is it borrowed as the two dialects `queue` writes, and `Framed` flattens MySQL and
MariaDB. `postgres()` and `framed()` are the two gates over one `endpoint()`, which skips SQLite
and SQL Server by asking `queue::migration`. `rendered` is what the binary protocol costs: MySQL
answers a `bigint` as octets where PostgreSQL renders it, so a framed row is read against the
column definition it arrived under and rendered to the text both protocols agree on.

**Three cases are new and all three ran against a real server**: § 2's schema and § 4's claim
(the split's lock, `attempts + 1`, the lease written by value), § 6's two write-backs keyed on the
lease from both sides, and § 6's move as a pair inside one transaction. `python tools/db-matrix.py
--driver postgres --driver mysql --driver mariadb` is 3/3 ok.

**Two things in the file are still PostgreSQL's and deliberately so.** `orders` builds § 3's
application table with `bigserial`, and its two cases are `postgres()`-gated; `push` and `claim`
branch on `Conn::driver()` and every other case's ad-hoc SQL is written in the driver's own
placeholder spelling through `Conn::text()`.

**The driver's stage-7 check is unchanged and still correctly filed.**
`mysql_and_mssql_reset_through_the_protocol_and_lose_theirs` needs SQL Server and there is no TDS
driver — `TdsConn` is a busy-state cell. Do not rename it or write a MySQL-only test under it.

**`[context]` gap:** the manifest's `adrs` carries no ADR 0084 section, and every item below cites
one. The statements' own doc comments carried it this time; add `0084 §2`, `§4`, `§6` before a
session has to write a *new* statement rather than run a landed one.

## Next group

**One file set: `crates/nvs-stdlib/tests/queue.rs`, with `crates/nvs-stdlib/src/queue.rs` read at
the constant each item names.** Every case skips with no `NVS_DB_MATRIX_DRIVER`, exactly as the
nine already there do. Take them in any order — they share the helpers and touch nothing else.

- [ ] **A framed dedupe push is refused by the index, not by the guard** (0084 § 2, gap 3).
      `crates/nvs-stdlib/src/queue.rs:597` is `INSERT_MYSQL`, whose `first` is the dedupe read this
      file has never issued: `dedupe_pending = ?` reaches `MIGRATION_MYSQL`'s stored generated
      column, which is a construct only a server can refuse. `crates/nvs-stdlib/tests/queue.rs:505`
      is `push`, which binds `None` there and needs a keyed sibling to reach the pair at all.
- [ ] **§ 4's visibility arm on the framed dialect** (0084 § 4).
      `crates/nvs-stdlib/tests/queue.rs:576`'s `claim` sends `cutoff` and every framed case so far
      passes `0`, so `CLAIM_MYSQL`'s `(state = 1 and claimed_at <= ?)` arm
      (`crates/nvs-stdlib/src/queue.rs:625`) has never matched a row. Assert the bound from both
      sides, as `a_visibility_timeout_returns_an_abandoned_job_to_the_queue` does for PostgreSQL.
- [ ] **§ 5's roster answers on the framed dialect, and say what the other three cost** (0084 § 5).
      `crates/nvs-stdlib/src/queue.rs:670`'s `QUEUES_MYSQL` is `pub` and reachable now.
      `STATUS_MYSQL` (`crates/nvs-stdlib/src/queue.rs:813`), `CANCEL_MYSQL` (`:844`) and
      `COUNTS_MYSQL` (`:885`) are private, as their PostgreSQL twins are, so an integration test
      cannot name them: either they go `pub` in both dialects beside `QUEUES_*`, or the case is an
      in-crate `#[cfg(test)]` one. Decide it in the item and write down which.

## Backlog

- `nvs-cli`'s worker has no real-server leg at all; its readers live in a binary crate, so one
  needs a lib target first — ADR 0132 § 1 owns that boundary.
- `orders`' `bigserial` is § 3's last PostgreSQL-only construct in the test file; splitting it the
  way `MIGRATION_MYSQL` splits § 2's is what a framed § 3 case would need.
- `tools/db-matrix.py`'s `SUITES` is two entries; a third would be where a `nvs-cli` leg lands.
- SQL Server: no TDS driver, so § 13's reset and every queue statement stay unasserted there —
  `crates/nvs-db/src/conn.rs`'s `TdsConn` is the whole of it.
