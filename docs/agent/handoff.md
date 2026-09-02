# Handoff

## State

**MySQL runs one statement.** `crates/nvs-db/src/mysql.rs` holds ADR 0067 § 1's two round trips:
`prepare` over `COM_STMT_PREPARE`, `execute` over `COM_STMT_EXECUTE`'s binary protocol, and
`start_statement` sequencing them behind § 4's busy state. `read_ok` is now a wrapper over
`read_answer`, which answers `Answer::Done { affected, last_id }` or `Answer::Columns(n)` — the
module doc owns the four packet shapes and how a `0x00` status is told from a zero column count.

**It stops at the first row, deliberately.** `read_columns` takes the definitions, whose count is
known, and returns with the connection `State::Streaming`. Decoding a binary row against § 9's type
map is the next slice; `MySqlConn::query` hands back `mysql_common`'s own `Column` because choosing
the Novis type per column is that slice's decision, not this one's.

**No statement cache yet**, so every statement costs both round trips — § 1 prices it that way and
`crate::pg`'s `StatementCache` is the shape to copy. `close_statement` was deliberately *not*
written: `COM_STMT_CLOSE` has no caller until eviction exists, and shipping it dead would have been
a body nobody had run.

**`crate::pg::second_statement` is now `pub(crate)`** so both drivers refuse a second statement with
one sentence. That is the only pg.rs change.

**The driver's stage-2 acceptance check was mis-filed, and one third of it is now closed.**
`a_connect_named_private_endpoint_needs_no_net_connect_grant` is landed in
`crates/nvs-stdlib/src/db.rs` and the name moved to the `-p nvs-stdlib` check;
`a_db_open_target_in_a_denied_range_fails` moved with it, since both are questions about a
deployment and `nvs_types::intrinsics`' known gap 6 says no `-p nvs-types` test can ask one. The
`-p nvs-types` check keeps `a_host_on_a_sqlite_settings_literal_is_a_compile_error` alone, which is
genuinely its own — and both remaining names wait on `Core\Db::open`, blocked on a registry type for
a shape **parameter** (`nvs_stdlib::db` known gap 1). That is a language-surface decision and wants
its own ADR, not a slice. `docs/agent/goals/5-database.toml` was re-copied from the live file.

**`orient.py` gaps:** `[context] adrs` wants ADR 0067 § 4 (the busy state's contract) and § 5 (the
placeholder rewriter) for the slices below; § 1, § 3, § 9 and § 13 are already there and were enough
for this one.

## Next group

**MySQL's rows and its resolve — one file set: `crates/nvs-db/src/mysql.rs`,
`crates/nvs-db/src/conn.rs` and `crates/nvs-db/src/pg.rs`. `crates/nvs-db/src/pg.rs:2676`
(`start_statement`) and its `PgRows` are the shape all three copy, and
`crates/nvs-db/src/mysql.rs:1310` (`start_statement`) is the function they extend.**

- [ ] **A binary row decodes against § 9's type map** — ADR 0067 § 9. The result set
      `crates/nvs-db/src/mysql.rs:1310` stops in front of: read a row packet, its null bitmap, and
      each value in the type its `Column` declares. `crates/nvs-db/src/mysql.rs:1198` (`read_columns`)
      is what hands over the metadata, and `crates/nvs-db/src/pg.rs:2676`'s `PgRows` is the borrowing
      shape to mirror.
- [ ] **`MySqlTarget::resolve` off a `[db.<name>]` block** — ADR 0067 § 2, mirroring
      `crates/nvs-db/src/pg.rs:375`'s `PgTarget::resolve`. Nothing above this crate can reach a MySQL
      connection until this exists; `crates/nvs-db/src/lib.rs`'s module doc records that gap.
- [ ] **§ 1's statement cache on this connection** — ADR 0067 § 1 and § 13. `crate::pg`'s
      `StatementCache` is already the shared shape; add `COM_STMT_CLOSE` for eviction beside
      `crates/nvs-db/src/mysql.rs:1129` (`execute`), and § 13's reset must invalidate it because
      `COM_RESET_CONNECTION` drops prepared statements — `crates/nvs-db/src/mysql.rs:1259`
      (`reset_session`) is where that has to happen.

## Backlog

- `Core\Db::open` and `Db\Settings` need a registry type for a shape *parameter* — a language-surface
  decision wanting its own ADR (`nvs_stdlib::db` known gap 1 states it).
- Two stage-2 acceptance names stay open behind that: `a_host_on_a_sqlite_settings_literal_is_a_compile_error`
  and `a_db_open_target_in_a_denied_range_fails` (`docs/agent/loop-goal.toml`).
- The other three drivers have no connect path at all (`nvs_stdlib::db` known gap 2).
- ADR 0067 § 11's `query` span is not opened on the MySQL path yet; `crates/nvs-db/src/span.rs` owns
  the shape.
