# Handoff

## State

**Goal 9 stage 3's emitters are on disk.** `crates/nvs-db/src/ddl.rs` writes a `CREATE TABLE` — with
its primary key, its unique constraints and its indexes — in all four dialects, keyed on
`nvs_db::Dialect` and never on a driver. Four named tests are green:
`every_construct_in_the_vocabulary_emits_in_all_four_dialects`,
`the_emitters_follow_dialect_rather_than_driver`,
`mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists` and
`mysql_gives_an_indexed_text_column_a_prefix_length`. The module's own doc owns every design call it
made — why MySQL writes `BIT(1)` and not `BOOLEAN`, why an index is inside its table on that one
backend, and the four round-trips that are lossy and are § 5's input rather than this module's bug.

**Stage 3 is two thirds done.** What `ddl.rs` has no word for yet is `ALTER`, and with it the plan,
the step, the three grades and the reason a step carries — the group below.

**`examples/schema.nvs` is on disk and does not compile yet, on purpose.** It is stage 6's target, and
**the driver will name it every session until stage 6 lands; that is the ordinary state, not a
regression** — see the playbook bullet for what it masks (every `cargo-named` check of this goal, since
program checks run first). `nvs.toml` still owes it an `[[app]]` entry with `connect = ["schema"]`, the
`db.schema` grant and a `[db.schema]` SQLite block; that file is one snapshot every fixture loads, so
it goes in with the capability at stage 6 rather than earlier.

**A `[context]` gap:** the pack printed ADR 0145 § 8 alone. The next group needs §§ 5, 6 and 7 as well
— add them to `[context] adrs` in `docs/agent/loop-goal.toml`, because `orient.py` slices a section
live and naming them here does not make it print them.

## Next group

**Stage 3.3, the plan as a document** — one file set: `crates/nvs-db/src/ddl.rs` and a new
`crates/nvs-db/src/plan.rs` beside it. Both slices need ADR 0145 §§ 6 and 7 read first; § 8 is already
implemented by the file you are extending.

- [ ] **`ALTER TABLE`, and a step that carries its own SQL** — goal stage 3.3, ADR 0145 §§ 6 and 8.
      `every_step_carries_terminated_executable_sql_including_the_ones_apply_refuses`: a step exposes
      its grade, the reason for that grade, and complete terminated SQL *including* the steps the
      applier refuses, because the plan is what a DBA pastes. Anchors:
      `crates/nvs-db/src/ddl.rs:100` (`create_table`, whose `Vec<String>` of terminated statements is
      the shape a step's SQL already takes), `crates/nvs-db/src/ddl.rs:169` (`column_type`, which an
      `ADD COLUMN` reuses whole), `crates/nvs-db/src/ddl.rs:408` (`literal`),
      `crates/nvs-db/src/sql.rs:72` (`Dialect`).
- [ ] **SQLite's create-copy-drop-rename rebuild**, graded `Destructive` unconditionally — the goal's
      § *Standing decisions* pre-authorizes it, ADR 0145 § 8.
      `sqlite_rebuilds_the_table_for_an_alter_it_cannot_express`: SQLite's `ALTER TABLE` adds, renames
      and drops a column and essentially nothing else, so everything else is a data copy. Anchors:
      `crates/nvs-db/src/ddl.rs:100` (`create_table` — the rebuild's second statement *is* a
      `CREATE TABLE` of the new shape), `crates/nvs-db/src/ddl.rs:385` (`rowid_identity`, which the
      copy has to preserve), `crates/nvs-db/src/schema.rs:483` (`Table`).

## Backlog

- Stage 4's five introspectors, answering the same value the builder produces — ADR 0145 § 4.
- `nvs.toml` owes `examples/schema.nvs` its `[[app]]` block, at stage 6 — this handoff's § State.
- The four lossy round-trips are stage 5's normalization input, not a bug — `crates/nvs-db/src/ddl.rs`
  module doc, *Known gaps* 1.
- `Table` enforces that an identity is *in* the primary key, not that it is the whole of one; SQLite's
  rowid form needs the stronger rule — `crates/nvs-db/src/ddl.rs` module doc, *Known gaps* 3.
- SQL Server cannot index an unbounded text column at all; the step is emitted and § 5's grading is
  where it becomes a refusal — same doc, *Known gaps* 2.
