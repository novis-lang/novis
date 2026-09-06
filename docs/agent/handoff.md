# Handoff

## State

**Goal 9 stage 4's readers are whole and proved against a real server.**
`crates/nvs-db/src/catalog.rs` now carries `ColumnRow`, `IndexRow` and `assemble`: the two reads'
rows, in `Read::row`'s positions, become one `Schema` through the same builders a declared schema
goes through, so an introspection cannot mint a column `crate::ddl` would refuse to emit.
`every_driver_introspects_into_the_same_schema_value_shape` asserts that as an **agreement across
all five drivers** over `ddl`'s own spellings, with the rows gapped and reversed on purpose;
`a_schema_applied_to_sqlite_assembles_back_into_itself` runs it against a real in-memory SQLite and
asserts a **fixed point** — read, emit, apply, read again, unchanged.

**Three divergences between the value applied and the value read are pinned rather than hidden**,
in that second test: table order (by name, not by declaration), SQLite's minted
`sqlite_autoindex_…` for an inline `UNIQUE`, and `INTEGER PRIMARY KEY` erasing a declared `int64`
width. Each is § 5's to normalise, and closing one fails that test on purpose.

**Stage 4 is still not green**, and not because of the crate: its second check is
`nvs schema dump --connection main`, which is stage 6's CLI. `examples/schema.nvs` is the driver's
reported failure for the same reason — `Core\Db\Schema` has no `fromArray` yet, and `nvs.toml`
still owes it an `[[app]]` with `connect = ["schema"]`, the `db.schema` grant and a `[db.schema]`
SQLite block.

Nothing in the orientation pack was missing.

## Next group

**Stage 5's diff** — one file set: `crates/nvs-db/src/plan.rs` for the diff, with
`crates/nvs-db/src/schema.rs` and `crates/nvs-db/src/catalog.rs` read beside it. The finding that
shapes the group: **`plan.rs` holds `Change`, `Step`, `Grade` and `Plan` and nothing that produces
one** — every type § 6 names is on disk and the function § 5 specifies is not.

- [ ] **The diff is one function over two `Schema`s** —
      `the_diff_compares_normalized_values_and_never_sql_text`,
      `a_table_the_schema_does_not_declare_is_reported_and_never_dropped` and
      `a_column_the_schema_does_not_declare_is_reported_and_never_dropped`, ADR 0145 § 5. The
      report-never-drop rule is already written, in `Schema`'s own doc. Anchors:
      `crates/nvs-db/src/plan.rs:121` (`Change`, the case set the diff emits),
      `crates/nvs-db/src/plan.rs:238` (`Step`), `crates/nvs-db/src/plan.rs:293` (`Plan`),
      `crates/nvs-db/src/schema.rs:679` (`Schema::new`), `crates/nvs-db/src/schema.rs:663`
      (`Schema`'s doc, § 7's rule).
- [ ] **Normalisation closes the three divergences stage 4 pinned** —
      `an_implicit_index_a_unique_constraint_created_is_not_a_difference`, ADR 0145 § 5. The three
      are asserted by name at `crates/nvs-db/src/catalog.rs:1818`, and that test's last three
      asserts are what change when the diff sees through them. Anchors:
      `crates/nvs-db/src/catalog.rs:1818`, `crates/nvs-db/src/plan.rs:121`.
- [ ] **SQLite's empty plan, against a real server** —
      `an_applied_schema_introspects_back_to_an_empty_plan_on_sqlite`, § 5's acceptance property on
      the one backend needing no container. The fixture already exists: `applied_and_read` at
      `crates/nvs-db/src/catalog.rs:1797` applies a schema and hands back the assembled value.
      Anchors: `crates/nvs-db/src/catalog.rs:1797`, `crates/nvs-db/src/plan.rs:293`.

## Backlog

- The other four `an_applied_schema_introspects_back_to_an_empty_plan_on_*` need Docker —
  `docs/agent/loop-goal.toml:4544`.
- Stage 5's eight grade tests, none written — `docs/agent/loop-goal.toml:4559`.
- `unquote`'s unquoted fallback is MySQL's alone; on the other three an unquoted spelling is an
  expression and is read as text — gap 4 of `crates/nvs-db/src/catalog.rs`'s module doc.
- Stage 4's `nvs schema dump` check waits on stage 6's CLI — `docs/agent/loop-goal.toml:4533`.
- `Core\Db\Schema::fromArray`, the `db.schema` grant and `nvs.toml`'s `[db.schema]` block are what
  `examples/schema.nvs` waits on — stage 6, `docs/agent/loop-goal.toml:4581`.
