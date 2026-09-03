# Handoff

## State

**M8 goal 5, stage 7. SQL Server caches a plan and loses it to the reset.**
`crates/nvs-db/src/tds.rs:3987`'s `start_statement` now takes ADR 0067 § 1's cache and picks the
request from it: a hit sends `sp_execute` (proc 12) with the handle and the values and no SQL, a
miss sends `sp_prepexec`, and an eviction sends `sp_unprepare` (proc 15) *before* the prepare that
needed the room. `crates/nvs-db/src/tds.rs:4086`'s `reset_session` is § 13's reset — MS-TDS spells
`sp_reset_connection` as `Status::RESET_CONNECTION` on a message of its own, whose answer is what
proves it landed — and it empties the cache, MySQL's asymmetry for the same protocol reason.
`crates/nvs-db/src/conn.rs:772`'s `TdsConn` carries the cache and `TdsConn::reset` consumes itself
like `MySqlConn::reset`. The standing acceptance failure is closed.

**The handle is filed by the stream, not the caller.** It arrives in a `RETURNVALUE` after the rows
and a statement with no result set has already ended when `read_rows` returns, so `TdsRows` borrows
the cache through a `Filing` and commits in `end()`; that type's doc owns it.

**§ 1's key is one component short on this protocol, and `crates/nvs-db/src/tds.rs:3939`'s
`TdsPlan` is the answer.** A plan compiled against `nvarchar(4000)` *truncates* a longer value
rather than refusing it, so the plan carries the `@params` it was compiled against, a hit whose
declaration no longer fits is unprepared and prepared again, and `StatementCache::forget` in
`crates/nvs-db/src/sql.rs` drops the entry rather than shadowing it.

**One gap is open on purpose**, unchanged: a parameter that is not UTF-8 is still refused by its
marker, so § 9's `bytes` has no SQL Server encoding and `nvs_stdlib::db::rendering_for` still
answers `None` for `Driver::SqlServer` — nothing above can reach the driver yet.

## Next group

**One file set: `crates/nvs-db/src/conn.rs` and `crates/nvs-db/src/tds.rs`, with
`crates/nvs-stdlib/src/db.rs` for the layer above.** `crate::mysql`'s arms in `conn.rs` are the
shapes to copy, and the playbook's `PgConn` trap is why each sequencing function stays free and
generic in the stream.

- [ ] **`Driver::SqlServer` runs a statement end to end from `conn.rs`** (0067 §§ 4 and 5).
      `crates/nvs-db/src/conn.rs:772`, `crates/nvs-db/src/conn.rs:863`,
      `crates/nvs-db/src/tds.rs:3987`, `crates/nvs-db/src/tds.rs:4178`,
      `crates/nvs-stdlib/src/db.rs:4303`. The enum's `query`/`execute` arms reach
      `tds::start_statement` with `&mut conn.cache`, and `rendering_for` gains
      `Driver::SqlServer` with `Dialect::SqlServer` so § 5's rewrite reaches the driver at all.
- [ ] **The pool's reset and destroy arms reach `TdsConn::reset`** (0067 § 13).
      `crates/nvs-db/src/conn.rs:863`, `crates/nvs-db/src/tds.rs:4178`. The free function and the
      consuming method are on disk; what is missing is the `Connection` arm, so
      `a_failed_reset_destroys_the_connection_rather_than_returning_it` covers this backend too.
- [ ] **§ 9's `bytes` gets a SQL Server encoding, or the refusal is pinned as the answer**
      (0067 § 9). `crates/nvs-db/src/tds.rs:3670`, `crates/nvs-db/src/tds.rs:3739`,
      `crates/nvs-stdlib/src/db.rs:4303`. `varbinary` needs its own `TYPE_INFO` and its own
      `@params` spelling; `text_param` is the one place both forms are decided.

## Backlog

- § 7's transactions on TDS: `BEGIN`/`COMMIT` as `PacketType::TransactionManager` — docs/adr/0067 § 7.
- A real SQL Server leg in `tools/db-matrix.py` — docs/plan/m8.md's five-driver matrix.
- `TdsTarget::time_zone` governs decoding only, so § 9's zone-less rows need their own case.
- MariaDB and MySQL share `reset_session`; SQL Server's is its own — no third spelling wanted.
