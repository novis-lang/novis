# Handoff

## State

**`Core\Db\Column` is registered and `Core\Db\Rows::columns()` answers it**, so spec § 18's Results
table is landed whole for a buffered result: three readers over three slots, built one object per
described column beside the rows at query time. `ROWS_COLUMNS_SLOT`'s doc comment owns why that is
eager where the hydration class in the slot before it is lazy — a description is per *statement* and
bounded by the `select` list, and `nvs_db::PgRows` lends its row description out of the borrow the
rows are read from, so it is captured there or not at all.

**A `Core` enum needed no new machinery, which was the open question and is now answered by three
cases rather than by a decision.** An enum *is* its case's ordinal at runtime (ADR 0010), which
`Core\Cli::colorDepth` already answered with; `column_type_value` looks that ordinal up in the
registered table rather than writing the fourteen numbers a third time, and
`every_column_type_case_is_named` holds the two halves total in both directions. A conformance case
can spell `Core\Db\ColumnType::Json` and compare it with `==` — the first in the corpus to spell a
`Core` enum case at all, so that spelling is now known to compile.

**`nullable()` answers `true` on every column and says so on its own card.** A PostgreSQL
`RowDescription` carries no NOT NULL flag, and the catalog lookup that would is the per-statement
round trip ADR 0067 § 9's type map is written to avoid. The alternative — dropping a reader § 18
names, or answering `?bool` — would put a third answer in front of every caller to tell them
nothing.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded
`Core\Queue` (ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s
module doc owns why.

## Next group

**Stage 3's two named `nvs-db` tests — § 4's streaming half — and the file set is
`crates/nvs-db/src/pg.rs` alone.** Nothing in `nvs-stdlib` moves; `stream`'s registry rows are the
group after this one, and they are a bigger question (a cursor that holds the connection).

- [ ] **`a_large_result_streams_at_constant_memory`** — § 4's one member that does not buffer,
      asserted as a memory bound rather than as a row count. `crates/nvs-db/src/pg.rs:2374` is
      `PgRows` and `crates/nvs-db/src/pg.rs:2514` its `next_row`, which is already the
      row-at-a-time read `queried_rows` drains; what the test needs is the bound on both sides,
      measured the way `crates/nvs-stdlib/tests/allocation_policy.rs` measures one — the playbook's
      bullet on `nvs_runtime::budget` is why a second `#[global_allocator]` is not it. ADR 0067 § 4.
- [ ] **`a_second_statement_on_a_busy_connection_is_a_logic_error`** — and **read the `[[check]]`
      block at `docs/agent/loop-goal.toml:2791` before writing anything**:
      `crates/nvs-db/src/pg.rs:4383` already holds
      `a_second_statement_on_a_busy_connection_writes_nothing_and_is_refused`, which pins the same
      claim under another name, and the playbook's bullet on stage 2's comment header is the trap
      that renaming one to the check's spelling is forbidden where that block says so. ADR 0067 § 4.

## Backlog

- `stream`/`streamAs` in `nvs-stdlib` — the cursor that holds the connection; spec § 18, ADR 0067 § 4.
- `close` and `Connection`'s three readonly properties as readers — `crates/nvs-stdlib/src/db.rs`'s
  known gap 5 is the list of what § 18 still owes there.
- The pool, ADR 0067 § 13 — Stage 7's five named `nvs-db` tests, and the reset is a security boundary.
- `Core\Queue`, ADR 0084 — Stage 8, and what the driver's acceptance line names.
- `queryAs<T>`'s three run-time refusals want compile-time codes — `crates/nvs-stdlib/src/db.rs`'s
  known gap 8; both bands the checker would take one from are full.
- `open` waits on a shape-*parameter* `CoreTy` — gap 1, and a language-surface decision rather than
  a database one.
