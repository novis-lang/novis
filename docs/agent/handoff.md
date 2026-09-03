# Handoff

## State

**M8 goal 5, stage 7. A program can now hold a SQL Server connection and run a statement on
it.** `crates/nvs-stdlib/src/db.rs:2982`'s `connect` and `crates/nvs-stdlib/src/db.rs:3373`'s
`open` each have a `Driver::SqlServer` arm — `TdsTarget::resolve`, `address_of`/`port_of`,
`TdsConn::connect`, `Connection::SqlServer` — line for line the MariaDB one beside it.
`crates/nvs-db/src/tds.rs`'s `DEFAULT_PORT` is 1433 and `TdsTarget` is re-exported at the crate
root beside the other three targets.

**`execute` runs on the driver too**, through `crates/nvs-stdlib/src/db.rs`'s new `tds_write`:
`sp_prepexec` carries a statement with no result set exactly as it carries one with, so the send
is `tds_rows`' and only the read of it differs. `lastId` is `null` there and that is the answer —
SQL Server puts no generated key in the token stream, which `TdsRows::affected`'s own doc records.

**Known gap 2's two rosters were renamed to what they now divide**: `ONE_STATEMENT`
(`crates/nvs-stdlib/src/db.rs:4669`) is the four drivers that run one statement over the driver's
own `query` — `query`, `queryAs`, `execute` — and `BEYOND_ONE_STATEMENT`
(`crates/nvs-stdlib/src/db.rs:4683`) is the three that reach `executeMany` and § 7. The old names
said *reading*, which stopped being the line the two fall either side of the moment `execute`
landed.

**§ 9's `bytes` bind is recorded as refused, not closed**, under its own heading in
`crates/nvs-db/src/tds.rs`'s module doc. Closing it makes `sp_prepexec`'s `@params` a function of
the values a call binds rather than of the statement, and § 1 keys the plan cache on SQL text plus
expansion arity — so it is a § 1 question about the key's shape, not a § 9 rendering one. The read
half of that row is whole.

**A real server was asked, and it found a driver bug the whole suite could not**: `login_ack`
read LOGINACK's `TDSVersion` little-endian and its own fixture wrote it that way, so every
connection was refused at the last step of a working handshake. Both halves now read it high half
first, and the playbook bullet added this session owns why no gate here could have caught it. A
five-line program against `tests/db/compose.yaml`'s SQL Server then opened, decoded `select 41 +
1` and counted an `execute`. The standing acceptance failure is still stage
9's `an_open_host_matching_no_grant_is_a_diagnostic`, `nvs_types::intrinsics`' known gap 6, and is
not this goal's to close.

## Next group

**One file set: `crates/nvs-db/src/tds.rs`, with `crates/nvs-stdlib/src/db.rs` for the arm each
one unlocks.** The first is a check `loop-goal.toml` already names; the other two are what empties
`BEYOND_ONE_STATEMENT`.

- [ ] **`mssql_resets_through_sp_reset_connection_and_loses_its_cache`** (0067 § 13).
      `crates/nvs-db/src/tds.rs:4865`, `crates/nvs-db/src/tds.rs:4718`,
      `crates/nvs-db/src/mysql.rs:3958`. Stage 7's check names this test and the playbook bullet
      saying it cannot be written is now stale: `reset_session` exists and the cache it empties is
      `TdsPlan`'s. MySQL's twin is the shape — assert the command went out, that the cache emptied
      with it, and that the same statement then costs the prepare again.
- [ ] **`executeMany` gains its SQL Server arm** (0067 § 4). `crates/nvs-db/src/tds.rs:4766`,
      `crates/nvs-stdlib/src/db.rs:5934`. § 4 is N executions on every driver, so the driver owes
      an `execute_many` over `start_statement`'s handle and the stdlib arm is three lines.
- [ ] **§ 7's commands, and the two rosters collapse back to one** (0067 § 7).
      `crates/nvs-db/src/tds.rs:4766`, `crates/nvs-stdlib/src/db.rs:4436`,
      `crates/nvs-stdlib/src/db.rs:4511`. `BEGIN TRAN`/`COMMIT`/`ROLLBACK` with `SAVE TRANSACTION`
      for a nested level, then `Transacting::SqlServer`. When it lands the two `const`s hold the
      same four drivers and `driverless` takes one roster again.

## Backlog

- § 9's `bytes` bind, whose real question is § 1's cache key — `crates/nvs-db/src/tds.rs`'s module doc.
- `crate::queue`'s four members are still PostgreSQL-only — `crates/nvs-stdlib/src/queue.rs`.
- SQLite has no `connect`/`open` arm and earns PostgreSQL's refusal — `crates/nvs-stdlib/src/db.rs` known gap 2.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` — `nvs_types::intrinsics` known gap 6.
