# Handoff

## State

**M8 goal 5, stage 7. SQL Server now runs every § 4 member that is a statement** — `query`,
`queryAs`, `execute` and, as of this session, `executeMany`. `crates/nvs-db/src/tds.rs:4880`'s
`execute_many` is `crate::mysql::execute_many`'s loop and its rules line for line: the busy check
ahead of the empty-list no-op, the arity agreement refused before anything is written, every set
attempted past a refusal with the **first** error reported, and a poisoned wire the one thing that
ends it early. It needs no request shape of its own — § 1's cache makes the first set an
`sp_prepexec` and every set after it an `sp_execute` — which is why the batch is `start_statement`
per set rather than a second sender. `TdsConn::execute_many` is the two-line delegation.

**Known gap 2's rosters now divide on § 7 rather than on one-versus-many statements.**
`RUNS_STATEMENTS` (`crates/nvs-stdlib/src/db.rs:4669`) is the four drivers behind every statement
member, `executeMany` included; `TRANSACTS` (`crates/nvs-stdlib/src/db.rs:4684`) is the three that
reach § 7. They collapse to one const the session § 7's commands land.

**The group's first item was already on disk and green** —
`mssql_resets_through_sp_reset_connection_and_loses_its_cache` — so what it owed was the stale
comment beside its check in both goal files, and that is written. The playbook bullet added this
session owns the check.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`,
`nvs_types::intrinsics`' known gap 6, and is not this goal's to close.

## Next group

**One file set: `crates/nvs-db/src/tds.rs` with `crates/nvs-db/src/conn.rs`, then
`crates/nvs-stdlib/src/db.rs`.** It is § 7 end to end — the driver's only remaining refusal.

- [ ] **§ 7's commands on TDS** (0067 § 7). `crates/nvs-db/src/tds.rs:4982`,
      `crates/nvs-db/src/conn.rs:772`, `crates/nvs-db/src/mysql.rs:2106`. `begin`, `commit`,
      `roll_back` and a `simple_command` that sends one T-SQL text as a `PacketType::SqlBatch` and
      drains it; `TdsConn` gains `depth: Cell<u32>` beside `state`, and the four two-line
      delegations follow `TdsConn::query`. MySQL's trio at that anchor is the shape, savepoint
      naming and depth accounting included (`crate::pg::savepoint_name`, `open_transaction`), with
      `SAVE TRANSACTION`/`ROLLBACK TRANSACTION <name>` for T-SQL's spelling and **no `RELEASE`,
      which SQL Server does not have** — a nested commit is therefore a no-op on the wire that only
      moves the depth. Two facts to decide before writing, both of them T-SQL's and neither MySQL's:
      **`SET TRANSACTION ISOLATION LEVEL` is session-scoped here**, not next-transaction, so an
      outermost level has to be put back when the outermost transaction ends — § 13's
      `sp_reset_connection` covers the *pool*, but not a second `transaction()` in the same request;
      and **SQL Server has no read-only transaction at all**, so `{readOnly: true}` is an
      `InvalidInput` naming the fix rather than a silently writable transaction.
- [ ] **`Transacting` gains its SQL Server arm, and the two rosters collapse to one** (0067 § 7).
      `crates/nvs-stdlib/src/db.rs:4439`, `crates/nvs-stdlib/src/db.rs:4514`,
      `crates/nvs-stdlib/src/db.rs:4669`, `crates/nvs-stdlib/src/db.rs:4684`. Four delegations and
      one `transacting` arm; then `RUNS_STATEMENTS` and `TRANSACTS` become one const and
      `the_refusal_names_every_driver_that_sends` asks one roster instead of two.
- [ ] **Known gap 2 and the two module docs say the driver is whole** (0067 § 4).
      `crates/nvs-stdlib/src/db.rs:88`, `crates/nvs-db/src/tds.rs:74`. Both currently name § 7 as
      the one remaining refusal; only after the two above.

## Backlog

- § 9's `bytes` bind stays refused, and closing it is a § 1 cache-key question — `crates/nvs-db/src/tds.rs`'s module doc.
- An `open` pool's bounds have no operator spelling — known gap 1, `crates/nvs-stdlib/src/db.rs`.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` — `nvs_types::intrinsics`' known gap 6.
- `crate::queue`'s four members are still PostgreSQL-only — known gap 2.
- No `loop-goal.toml` check names § 4's batch on SQL Server; the two new cases are unnamed acceptance.
