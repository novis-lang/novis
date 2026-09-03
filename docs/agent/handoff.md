# Handoff

## State

**M8 goal 5, stage 7. SQL Server reaches `Core\Db`'s read path, and nothing can hold a connection
to use it on.** `crates/nvs-stdlib/src/db.rs:4327`'s `rendering_for` binds through
`nvs_db::tds::encode` in `Dialect::SqlServer`, `queried_rows` has its arm into
`crates/nvs-stdlib/src/db.rs:4983`'s `tds_rows` — `mysql_rows`' shape a third time — and
`crates/nvs-stdlib/src/db.rs:5623`'s `tds_column_value` builds all five of § 9's structured rows,
where MySQL's twin builds three. `crates/nvs-stdlib/src/db.rs:4186`'s `warm_connection` resets a
`TdsConn` and only SQLite is dropped there now.

**`Core\Db::connect` and `open` have no SQL Server arm**, so every arm above is unreachable from a
program: nothing in `nvs-stdlib` calls `nvs_db::TdsConn::connect`, and the previous handoff's claim
that `rendering_for`'s `None` was "the whole of what stands between this and § 4 running" was
wrong about the tree. That arm is the next group's first item and both halves it needs —
`nvs_db::TdsTarget::resolve` and `TdsConn::connect` — are already written.

**Known gap 2 is a roster per member, not one list.** `READING` (four drivers) and `BEYOND_READING`
(three) sit above `crates/nvs-stdlib/src/db.rs:4662`'s `driverless`, which renders the caller's own
roster into the refusal; each of the four send members passes the one its `match` actually has.
`execute` is the one member that could be written against `nvs_db::tds::TdsConn::query` today —
that method's doc says `execute` *is* it, parted by `TdsRows::affected` — while `executeMany` and
§ 7's `transaction` have no `TdsConn` primitive at all.

**`crate::queue` no longer reads `rendering_for` as its own roster.** `no_dialect` splits on
`migration` and tells an operator which of two gaps they are waiting on: the queue's own for SQL
Server, `Core\Db`'s gap 2 for SQLite. The playbook bullet added this session owns why that coupling
was invisible.

**The driver's standing acceptance failure is stage 9's and is not this goal's to close.**
`an_open_host_matching_no_grant_is_a_diagnostic` is `nvs_types::intrinsics`' known gap 6: `Env`
carries no capability set and no capability is checked at check time.

## Next group

**One file set: `crates/nvs-stdlib/src/db.rs`, with `crates/nvs-db/src/tds.rs` for what it calls.**
The first item is what makes this session's three arms reachable; the second and third are the two
send members that follow from the same `TdsConn` surface.

- [ ] **`connect` and `open` open a SQL Server connection** (0067 §§ 2 and 3).
      `crates/nvs-stdlib/src/db.rs:2959`, `crates/nvs-stdlib/src/db.rs:3362`,
      `crates/nvs-db/src/tds.rs:207`, `crates/nvs-db/src/tds.rs:4950`. Both arms are the MariaDB
      one line for line — `TdsTarget::resolve(block)`, `address_of`, `TdsConn::connect`,
      `Connection::SqlServer` — except that `nvs_db::tds` has **no `DEFAULT_PORT`**; add it (1433)
      as `crates/nvs-db/src/mysql.rs:179` spells it. `open`'s arm is the `other =>` refusal whose
      comment currently says "binds and decodes but has no handshake here".
- [ ] **`execute` gains its SQL Server arm and leaves `BEYOND_READING`** (0067 § 4).
      `crates/nvs-stdlib/src/db.rs:5783`, `crates/nvs-stdlib/src/db.rs:4642`,
      `crates/nvs-db/src/tds.rs:2903`. `mysql_write`'s shape over `TdsConn::query`; `Written.changed`
      is `TdsRows::affected` and `last_id` is always `None` — SQL Server puts no generated key in
      the token stream, which `affected`'s own doc states.
- [ ] **§ 9's `bytes` bind closes or is recorded as refused** (0067 § 9).
      `crates/nvs-db/src/tds.rs:4619`, `crates/nvs-db/src/tds.rs:5017`. Every parameter goes out as
      one `nvarchar` and `varbinary` has no text form a cast recovers, so this is a typed parameter
      marker in `sp_prepexec`'s `@params` or it stays this driver's one open row.

## Backlog

- `executeMany` and § 7's `transaction` need `execute_many` and `begin`/`commit`/`roll_back` on
  `nvs_db::TdsConn` before an arm here is writable — `crates/nvs-db/src/tds.rs` owns that surface.
- The `mssql: ok` matrix leg (`docs/agent/loop-goal.toml:2969`) cannot pass until the handshake arm
  lands; `tools/db-matrix.py --driver mssql` is the runner.
- Stage 9's `an_open_host_matching_no_grant_is_a_diagnostic` — `nvs_types::intrinsics` known gap 6,
  and `a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted` behind it.
- `Core\Db\Rows::stream` is still unwritten on every driver — ADR 0067 § 4.
