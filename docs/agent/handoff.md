# Handoff

## State

**M8 goal 5, stage 7. SQL Server connects and describes a result set.**
`crates/nvs-db/src/tds.rs:2223`'s `Tokens::columns` reads `COLMETADATA` into
`crates/nvs-db/src/tds.rs:1923`'s `TdsColumn`, and `crates/nvs-db/src/tds.rs:2274`'s `type_info` is
MS-TDS § 2.2.5.4.1's tables written as a `match`: 36 type bytes, each measured by one of
`crates/nvs-db/src/tds.rs:1862`'s five `Length` families — the vocabulary the row reader will ask
its questions in. TDS 4.2's six spellings (`CHAR`, `VARCHAR`, `BINARY`, `VARBINARY`, `DECIMAL`,
`NUMERIC`) are refused by their byte rather than parsed on a guess, and the reasons are on
`Tokens::type_info` itself.

**`TdsColumn::column_type` classifies § 9 off the type byte alone**, which is the whole difference
from `crate::PgColumn::column_type`: SQL Server's `bit` *is* one bit, so PostgreSQL's `BIT(1)`
modifier question does not arise here. `ColumnType::Uint` and `ColumnType::Json` are both unreachable
on this backend, argued on that method rather than restated here.

**Nothing runs a statement yet**: no `ROW`, no PLP value reader, no cache, no reset. The standing
acceptance failure `mssql_resets_through_sp_reset_connection_and_loses_its_cache` is therefore
unchanged and is **not** a regression — it needs both of the first two slices below.

**§ 8's two absences on this backend are recorded on the fields they belong to, not here**:
`ServerError::driver_code` is `Option<u32>` (`crates/nvs-db/src/conn.rs:556`) because SQL Server's
number is a `LONG`, and `ServerError::sql_state` is empty because TDS sends no such field.

## Next group

**One file set: `crates/nvs-db/src/tds.rs`, with `crates/nvs-db/src/conn.rs` for the shared state.**
`crates/nvs-db/src/mysql.rs:2897`'s `MySqlRows` is the shape to copy, and the playbook's `PgConn`
trap is the reason each of these is a free function generic in the stream rather than a method.

- [ ] **`ROW`, `NBCROW` and PLP over `Wire::read_packet`, into a `TdsRows`/`TdsRow`** (0067 §§ 4
      and 9). `crates/nvs-db/src/tds.rs:723`, `crates/nvs-db/src/tds.rs:1862`,
      `crates/nvs-db/src/tds.rs:2223`, `crates/nvs-db/src/mysql.rs:2897`,
      `crates/nvs-db/src/mysql.rs:2358`. A packet and not a message, for the reason `Tokens`' own
      doc gives; `Length` already says how every value is measured, so this slice is the buffer that
      spans packets, the `NBCROW` null bitmap and PLP's chunks — not a second type table. The borrow
      of the wire is § 4's one-statement-at-a-time rule, and § 11's `QuerySpan` rides it as it does
      on the other two drivers.
- [ ] **§ 1's cache over `sp_prepexec`, then § 13's reset through `sp_reset_connection`** (0067
      §§ 1 and 13). `crates/nvs-db/src/conn.rs:772`, `crates/nvs-db/src/tds.rs:2519`,
      `crates/nvs-db/src/tds.rs:362`. This is the slice the acceptance check names.
      `Status::RESET_CONNECTION` is the reset without a round trip of its own, so decide between it
      and an RPC and say which. **Give the `cache` field a reader in the same slice**: a
      `pub(crate)` field nothing reads is a `dead_code` warning, which is why `TdsConn::packet_size`
      (`crates/nvs-db/src/tds.rs:2589`) exists.
- [ ] **`Driver::SqlServer` runs a statement end to end from `conn.rs`** (0067 §§ 4 and 5).
      `crates/nvs-db/src/conn.rs:772`, `crates/nvs-db/src/tds.rs:2546`,
      `crates/nvs-db/src/sql.rs:235`. The two slices above give the pieces; this is the enum arm and
      `sql.rs`'s `@p1` rewriting that make `Core\Db` reach them.

## Backlog

- The five-driver matrix leg for SQL Server, per ADR 0067's *Verification* — `tests/db/compose.yaml`.
- § 7's `SAVEPOINT` nesting on TDS, which no slice above reaches — ADR 0067 § 7.
- `sql_variant`'s value carries its own type description; the row reader may refuse it — 0067 § 9.
