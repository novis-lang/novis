# Handoff

## State

**M8 goal 5, stage 7. SQL Server connects.** `crates/nvs-db/src/tds.rs:2116`'s `TdsConn::connect`
opens the socket, `negotiate_tls` tunnels § 3's TLS inside PRELOGIN packets, and
`crates/nvs-db/src/tds.rs:2044`'s `login` sends LOGIN7 and reads the answer to its end — a refusal,
the `LOGINACK` that is the only acceptance, and the `ENVCHANGE` that settles the framing, all applied
after the whole message rather than during it. `TdsConn` holds the wire, § 4's busy state and § 9's
declared zone; the wire's own default type parameter is now `NvsTls<Tunnel<NvsTcp>>`, which is what
tunnelled TLS costs on this protocol.

**§ 8's two absences on this backend are decided and recorded on the fields they belong to, not
here.** `ServerError::driver_code` is `Option<u32>` (`crates/nvs-db/src/conn.rs:556`): SQL Server's
number is a `LONG` and `THROW`'s whole range starts at 50000, so a `u16` lost `driverCode` for every
error an application raised itself. `ServerError::sql_state` is **empty** on TDS, which sends no such
field, and both of its readers — `Display` and `nvs_stdlib`'s `sqlState` slot — treat empty as absent
so a program reads `null` rather than `""`. Inventing ODBC's five characters was refused for the
reason `kind_of` gives about its own table.

**Nothing runs a statement yet**: no `COLMETADATA`, no `ROW`, no cache, no reset. The standing
acceptance failure is therefore unchanged and is not a regression —
`mssql_resets_through_sp_reset_connection_and_loses_its_cache` needs all three slices below.

**Item [2] of the previous handoff was one item and is two slices.** The result set on this protocol
is TYPE_INFO for ~30 type ids, four length disciplines (fixed, `BYTELEN`, `USHORTLEN`, `PARTLEN`/PLP)
and the `NBCROW` null bitmap; that is the largest single piece left in the driver and does not fit
one session beside anything else. It is split below.

## Next group

**One file set: `crates/nvs-db/src/tds.rs`, with `crates/nvs-db/src/conn.rs` for the shared enums.**
`crates/nvs-db/src/mysql.rs:2897`'s `MySqlRows` and `crates/nvs-db/src/pg.rs:1023`'s `PgColumn` are
the two shapes to copy: a driver's column carries the server's *own* type description and a driver's
row carries the server's own bytes, because § 9's decode into a Novis value is `nvs-stdlib`'s and not
this crate's (the playbook's `loop-goal.toml`-check trap owns why).

- [ ] **TYPE_INFO and `COLMETADATA`, into a `TdsColumn` that classifies as `ColumnType`** (0067
      §§ 5 and 9). `crates/nvs-db/src/tds.rs:1758`, `crates/nvs-db/src/tds.rs:1777`,
      `crates/nvs-db/src/conn.rs:427`, `crates/nvs-db/src/pg.rs:1023`. Sans-IO over a payload like
      `Tokens` is, so it is unit-tested with no socket. § 9's `uniqueidentifier` is `Uuid`,
      `datetimeoffset` is `Instant`, `datetime2`/`datetime` are `DateTime`, and `CheckViolation`'s
      being unreachable has a twin here: `BIT` is `Bool` while `BIT(n>1)` is not a SQL Server type at
      all.
- [ ] **`ROW`, `NBCROW` and PLP over `Wire::read_packet`, into a `TdsRows`/`TdsRow`** (0067 §§ 4
      and 9). `crates/nvs-db/src/tds.rs:731`, `crates/nvs-db/src/mysql.rs:2897`,
      `crates/nvs-db/src/mysql.rs:3006`, `crates/nvs-db/src/mysql.rs:2358`. A packet and not a
      message, for the reason `Tokens`' own doc gives; the borrow of the wire is § 4's
      one-statement-at-a-time rule, and § 11's `QuerySpan` rides it as it does on the other two.
- [ ] **§ 1's cache over `sp_prepexec`, then § 13's reset through `sp_reset_connection`** (0067
      §§ 1 and 13). `crates/nvs-db/src/conn.rs:772`, `crates/nvs-db/src/tds.rs:2089`,
      `crates/nvs-db/src/tds.rs:360`. This is the slice the acceptance check names. `Status::
      RESET_CONNECTION` is the reset without a round trip of its own, so decide between it and an
      RPC and say which. **Give the `cache` field a reader in the same slice**: a `pub(crate)` field
      nothing reads is a `dead_code` warning, which is why `TdsConn::packet_size` exists.

## Backlog

- The four drivers' `driver_code` is `u32` now; only `nvs-stdlib`'s `i64::from` reads it, and no
  spec text moved — 0067 § 8 already said `?int`.
- `tests/handshake.rs` has no SQL Server leg, and `tools/db-matrix.py` has a server to run one
  against — after the result set, per ADR 0067's *Verification*.
- § 7's `begin`/savepoints on TDS are a `TransactionManager` request, not text — `crates/nvs-db/src/tds.rs:294`.
- Stage 9 (§ 10's three diagnostics, ADR 0024 § 4's pair, § 11 whole) is untouched — docs/agent/loop-goal.toml.
