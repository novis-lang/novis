# Handoff

## State

**M8 goal 5, stage 7. SQL Server writes a statement as well as reading one.**
`crates/nvs-db/src/tds.rs:3582`'s `sp_prepexec_request` builds ADR 0067 § 1's RPC — `ALL_HEADERS`,
the procedure id, a by-reference `@handle`, the `@params` declaration, § 5's rewritten SQL and one
argument per marker — and `crates/nvs-db/src/tds.rs:3781`'s `start_statement` sends it and hands back
the `TdsRows` the previous slice built. Its own doc owns the two decisions: every parameter goes out
as `nvarchar` and the server casts it (`crate::mysql::execute`'s account, reached through another
protocol), and `sp_prepexec` rather than `sp_executesql` because § 1 keeps a cache and only this one
answers with a handle.

`RETURNSTATUS` and `RETURNVALUE` are read; `crates/nvs-db/src/tds.rs:2906`'s `TdsRows::returned`
keeps the last of them, which for `sp_prepexec` is the handle. `Done` gained `in_proc` — the
playbook's new bullet owns why, and it is a fix to the *reader* this slice would otherwise have
tripped over.

**Two gaps are open on purpose.** A parameter that is not UTF-8 is refused by its marker: § 9's
`bytes` maps to `varbinary`, and `nvarchar` → `varbinary` on SQL Server reinterprets UCS-2 rather
than parsing, so the driver has no honest encoding for one yet — `nvs_stdlib::db::rendering_for`
still answers `None` for `Driver::SqlServer`, so nothing above can reach this. And nothing files the
handle, so every execution prepares a plan that lives until the connection closes; the next item
closes it.

The standing acceptance failure `mssql_resets_through_sp_reset_connection_and_loses_its_cache` is
**unchanged and not a regression** — it is the next item whole, both halves of its `_and_`.

## Next group

**One file set: `crates/nvs-db/src/tds.rs`, with `crates/nvs-db/src/conn.rs` and
`crates/nvs-db/src/sql.rs` for the shared state.** `crates/nvs-db/src/mysql.rs:1814`'s
`cached_statement` and `crates/nvs-db/src/mysql.rs:1628`'s `reset_session` are the shapes to copy,
and the playbook's `PgConn` trap is why each stays a free function generic in the stream.

- [ ] **§ 1's cache keyed on SQL plus arity, and § 13's reset through `sp_reset_connection`**
      (0067 §§ 1 and 13). `crates/nvs-db/src/tds.rs:3781`, `crates/nvs-db/src/tds.rs:3582`,
      `crates/nvs-db/src/tds.rs:2906`, `crates/nvs-db/src/conn.rs:772`,
      `crates/nvs-db/src/sql.rs:290`. `StatementCache<i32>` on `TdsConn`; a hit sends `sp_execute`
      (proc 12) with the handle and the values and no SQL, an eviction sends `sp_unprepare`
      (proc 15), and the reset is an argument-less RPC of proc 16 that clears the cache — MySQL's
      asymmetry, for the same protocol reason. **The wrinkle worth knowing before you start:** the
      handle arrives in a `RETURNVALUE` *after* the rows, so `start_statement` cannot file it — either
      `TdsRows` borrows the cache and files it in `end()`, or the caller reads `returned()` once
      `next_row` has answered `None`. `sp_prepexec_request` already carries the per-argument writers
      (`int_param`, `text_param`, `declarations`) the other three procedures need.
- [ ] **`Driver::SqlServer` runs a statement end to end from `conn.rs`** (0067 §§ 4 and 5).
      `crates/nvs-db/src/conn.rs:772`, `crates/nvs-db/src/tds.rs:3781`,
      `crates/nvs-db/src/tds.rs:3842`. The `TdsConn` methods that delegate to the two free functions,
      then `nvs_stdlib::db`'s `rendering_for` gaining the driver.
- [ ] **§ 9's `bytes` gets a SQL Server encoding, or the refusal is pinned as the answer**
      (0067 § 9). `crates/nvs-db/src/tds.rs:3613` (`text_of`). A `varbinary` parameter needs its own
      `TYPE_INFO` in `text_param`'s sibling, and the encoder above has to say which values take it.

## Backlog

- § 7's transactions and § 13's pool on this driver — `docs/adr/0067-core-db.md` §§ 7 and 13.
- A `tests/db/compose.yaml` SQL Server leg and the matrix fields — `crates/nvs-db/src/matrix.rs`.
- `sql_variant` and `xml` values decode to nothing yet — `crates/nvs-db/src/tds.rs`'s module doc.
