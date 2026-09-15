# Handoff

## State

**Goal `m8-db-queue`, stage 6 is closed.** The three cases its `nvs-suite` check names are on disk
and green, and its three `cargo-named` checks were already so: `queryAs<T>`'s two doors are pinned
from Novis as well as from Rust.

`tests/conformance/core/db-query-as-hydrates-a-decimal-an-instant-and-bytes.nvst` fills a `decimal`
and a `tainted bytes` field from a live `:memory:` SQLite database and asks the third wire type the
erasure gives a codec type of its own — an `Instant` — of a column this backend cannot produce one
for, so `converted`'s class arm answers as a `ParseError` naming both class names
(`crates/nvs-stdlib/src/db/row.rs:440`). `nvs_db::SqliteColumn::column_type` answers `Instant` for no
declared type at all (`crates/nvs-db/src/sqlite.rs:444`), so the positive `TIMESTAMPTZ` half stays
the four wire drivers' and is asserted from `crates/nvs-stdlib/tests/db_stream.rs`.

`tests/conformance/core/db-query-as-calls-the-from-row-a-class-wrote-itself.nvst` pins the other
door: a class with no `#[Db\Derive]` that declares `fromRow` builds properties named after neither
column, through a connection and through a transaction alike.

Nothing is blocked. Stage 7's six named tests do not exist yet, which is the ordinary open state.

## Next group

**Stage 7: SQL Server's nullable unique key, at both doors** — one file set:
`crates/nvs-db/src/ddl.rs` and `crates/nvs-db/src/catalog.rs`. The goal's § *Standing decisions* is
what settles the semantics: a unique key's nulls are distinct on every backend, and SQL Server is
brought into line through its DDL rather than by the queue working around it.

- [ ] **`a_unique_key_over_a_nullable_column_is_a_filtered_index_on_sql_server` does not exist**, and
      stage 7's `-p nvs-db` check names it. A unique key is emitted inline as `CONSTRAINT … UNIQUE`
      (`crates/nvs-db/src/ddl.rs:172`) and after the fact as `ADD CONSTRAINT … UNIQUE`
      (`crates/nvs-db/src/ddl.rs:655`); on SQL Server a key over a nullable column becomes
      `CREATE UNIQUE INDEX … WHERE <column> IS NOT NULL` instead, both times.
      `rule:core-classes/queue-storage-is-a-table` owns the refusal it replaces — a `not null` column
      with a generated token stays refused.
- [ ] **`a_filtered_unique_index_reads_back_as_the_same_key` does not exist.** The catalog's index
      read is the one statement that reaches `sys` (`crates/nvs-db/src/catalog.rs:339`), and it
      already selects `has_filter`; what the test asks is that a key emitted by the slice above
      reads back as the same `UniqueKey` rather than as a plain index, so `plan` converges on a
      second run. `rule:core-classes/schema-introspection` owns the read.
- [ ] **`plan` refuses a nullable unique column it cannot read back as the same key** — the safe
      fallback the standing decisions name, at `crates/nvs-db/src/ddl.rs:645`, and only if the two slices
      above show the round trip does not close. It never emits DDL that will not converge.

## Backlog

- A closure reusing an enclosing `foreach`'s variable name is an ICE, not a diagnostic —
  `crates/nvs-ir/src/lower/expr.rs:2667`; the playbook bullet is the workaround, and the fix is
  `nvs_types`' capture set. Owner: `crates/nvs-ir`'s module doc.
- Stage 7's queue half — `queue_runs_on_every_driver` and the SQL Server dialect behind
  `nvs_stdlib::queue::runs` (`crates/nvs-stdlib/src/queue.rs:2373`) — after the DDL group above.
- Stage 7's `errors` array, bounded by attempts × the capped message
  (`crates/nvs-stdlib/src/queue.rs:882`), with `a_dead_lettered_row_carries_every_attempts_error`
  under `-p nvs-cli`.
- `nvs_types::derive` gap 2's subject, the `CodecTy::Opaque` field the reader has no case for, is
  gap 1's erasure and still open. Owner: `crates/nvs-types/src/derive.rs`'s module doc.
