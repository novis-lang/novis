# Handoff

## State

**M8 goal 5, stage 7. SQL Server reaches every member ADR 0067 declares.** § 7's `begin`, `commit`
and `roll_back` are on disk at `crates/nvs-db/src/tds.rs:5025` over a new `batch_command` — the
driver's only text path, and no caller's SQL ever reaches it — and `Transacting`
(`crates/nvs-stdlib/src/db.rs:4452`) has its fourth arm, so `transaction` runs on four drivers.

**Two T-SQL rules are this driver's alone and their doc comments are their home.** There is no
`RELEASE SAVEPOINT`, so a nested commit sends nothing and moves only the depth (a `COMMIT
TRANSACTION` there would commit the whole transaction). And `SET TRANSACTION ISOLATION LEVEL` is
*session*-scoped, so `TdsConn::isolation_moved` (`crates/nvs-db/src/conn.rs:814`) records a restore
that the outermost commit or rollback pays — and that the next outermost `begin` pays instead when a
refused commit ended the transaction before it could. `{readOnly: true}` is an `InvalidInput` naming
the fix: SQL Server has no read-only transaction at all.

**Known gap 2 is one roster again** — `HAS_A_DRIVER` (`crates/nvs-stdlib/src/db.rs:4669`), SQLite the
whole of it — and `driverless` no longer takes a roster argument. That argument is how `executeMany`
came to render the *transaction* roster and leave SQL Server out of a sentence about a member it runs.

**No TDS path is exercised against a real server yet**: `crates/nvs-db/tests/handshake.rs` has no SQL
Server leg at all, which is the next group. The standing acceptance failure is still stage 9's
`an_open_host_matching_no_grant_is_a_diagnostic`, `nvs_types::intrinsics`' known gap 6, and is not
this goal's to close.

`orient.py`'s map printed `nvs-stdlib`'s `json`, `registry` and `time` but not `db` — the goal's
central stdlib file. Its `[context] modules` needs `nvs-stdlib/src/db.rs`.

## Next group

**One file set: `crates/nvs-db/tests/handshake.rs` with `crates/nvs-db/src/matrix.rs`.** Every other
driver has a real-server leg and TDS has none, so what the unit tests cannot reach — that a real SQL
Server accepts these bytes at all — is unasserted from the handshake up.

- [ ] **SQL Server's handshake and one statement over a real server** (0067 §§ 3, 1).
      `crates/nvs-db/tests/handshake.rs:198`, `crates/nvs-db/tests/handshake.rs:230`,
      `crates/nvs-db/src/matrix.rs:107`. `mysql()`/`mysql_open` are the shape: an
      `NVS_DB_MATRIX_*` endpoint or an early `return`, which is the skip rule the whole crate
      shares. `tools/db-matrix.py` already points the mssql leg at a server with a `certs` leaf, so
      § 3's tunnelled TLS and `tls_ca_file` are what this first asserts.
- [ ] **§ 7's nesting on that server** (0067 § 7). `crates/nvs-db/tests/handshake.rs:568`,
      `crates/nvs-db/src/tds.rs:5025`. `a_mysql_transaction_nests_to_a_savepoint_and_rolls_back_to_it`
      is the twin to copy; what is new here is that the server accepts `SAVE TRANSACTION` and
      `ROLLBACK TRANSACTION <name>` as spelled, and that a nested commit really does leave the outer
      transaction open with nothing sent.
- [ ] **§ 13's reset from inside a transaction, and the level it puts back** (0067 § 13).
      `crates/nvs-db/tests/handshake.rs:780`, `crates/nvs-db/src/tds.rs:5193`. The reset clears both
      of § 7's counters; a real server is where "`sp_reset_connection` also puts the isolation level
      back" stops being this driver's claim about MS-TDS and becomes a measurement.

## Backlog

- A `bytes` parameter is still refused on TDS; closing it is a § 1 cache-key question — `crates/nvs-db/src/tds.rs`'s module doc owns why.
- SQLite is known gap 2's whole remainder: no connection path at all — `crates/nvs-stdlib/src/db.rs:3220`.
- `examples/transaction.nvs` runs against PostgreSQL only — `docs/agent/loop-goal.toml` stage 5.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` — `nvs-types`, known gap 6, another goal's.
