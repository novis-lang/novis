# Handoff

## State

**M8 goal 5, stage 8. SQL Server has a real-server leg.** `crates/nvs-db/tests/handshake.rs:1057`
asks that server the three facts the other three legs ask — `encrypt_option` from
`sys.dm_exec_connections` for § 3's tunnelled TLS, `SUSER_SNAME()` for the login LOGIN7 named, and
`18456` with an empty SQLSTATE for a password it cannot verify — and the first of them rides § 1's
`sp_prepexec`, so a green leg has carried a prepare, an execution and a result set through the
tunnel. `python tools/db-matrix.py --driver mssql` is green.

**§ 7 does not work against a real SQL Server, and the driver is why.** Writing the nesting twin
found it: the first statement after `TdsConn::begin` is refused with driver code **3989**, *"New
request is not allowed to start because it should come with valid transaction descriptor."* Once a
transaction is open the server hands back a descriptor in an `ENVCHANGE` of type 8, and every later
request must carry it in `ALL_HEADERS`; this driver drops types 8/9/10 into `EnvChange::Other`
(`crates/nvs-db/src/tds.rs:2389`) and writes a zero descriptor on every request. `tds.rs`'s scripted
peer cannot see this — it answers whatever the client wrote — which is exactly the gap a real server
was added to close. **The nesting test is not on disk**: it fails for a driver reason, and a red leg
would read as a fixture regression against stage 6's `mssql: ok`. It is the second item below and
the `##`-table note it needs is in that item.

The standing acceptance failure is still stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`,
`nvs_types::intrinsics`' known gap 6, and is not this goal's to close.

`orient.py`'s map still prints `nvs-stdlib`'s `json`, `registry` and `time` but not `db`; the goal's
`[context] modules` needs `nvs-stdlib/src/db.rs`.

## Next group

**One file set: `crates/nvs-db/src/tds.rs` with `crates/nvs-db/tests/handshake.rs`.** The first item
is the driver fix the second and third items are unwritable without.

- [ ] **Every request after a `BEGIN` carries the transaction descriptor** (0067 § 7).
      `crates/nvs-db/src/tds.rs:1740` (`EnvChange`, which needs a `Transaction` arm),
      `crates/nvs-db/src/tds.rs:2368` (`env_change`, where types 8/9/10 fall into `Other` — the new
      value is a `B_VARBYTE`, eight octets for a begin and empty for a commit or a rollback),
      `crates/nvs-db/src/tds.rs:4341` (`ALL_HEADERS_BYTES` and the `all_headers` that writes the
      zero), `crates/nvs-db/src/tds.rs:4429` (`rpc_header`, which every request body goes through),
      `crates/nvs-db/src/conn.rs:772` (`TdsConn`, where the descriptor sits beside `depth` as a
      `Cell<u64>` the token reader can write to, the way `state` already is). A reset must clear it
      with `depth`, at `crates/nvs-db/src/tds.rs:5465`.
- [ ] **§ 7's nesting on that server** (0067 § 7). `crates/nvs-db/tests/handshake.rs:1126` — the
      twin of `a_mysql_transaction_nests_to_a_savepoint_and_rolls_back_to_it`
      (`crates/nvs-db/tests/handshake.rs:651`), three inserts arranged so a bare `ROLLBACK
      TRANSACTION` answers `3` rather than `1,3`. The table must be a `##` global one: § 1 sends
      every statement through `sp_prepexec`, and a `#temp` created inside dynamic SQL dies with that
      batch. `STRING_AGG(CAST(id AS varchar(11)), ',') WITHIN GROUP (ORDER BY id)` is
      `GROUP_CONCAT`'s spelling here. An `mssql_run` helper goes beside
      `crates/nvs-db/tests/handshake.rs:452`.
- [ ] **§ 13's reset from inside a transaction, and the level it puts back** (0067 § 13).
      `crates/nvs-db/tests/handshake.rs:1126`, over `crates/nvs-db/src/tds.rs:5465`. What only a
      server can say is that `sp_reset_connection` rolled the open transaction back and that
      `DBCC USEROPTIONS` reports the login's isolation level again.

## Backlog

- Stage 6's `nvs-db` check names no SQL Server handshake test — `docs/agent/loop-goal.toml:2976`.
- `[context] modules` is missing `nvs-stdlib/src/db.rs` — `docs/agent/loop-goal.toml`.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` — `nvs_types::intrinsics`' known gap 6.
- Known gap 2's remainder is SQLite alone — `crates/nvs-stdlib/src/db.rs:4669`.
- SQL Server has no read-only transaction, so `{readOnly: true}` is an `InvalidInput` — ADR 0067 § 7.
