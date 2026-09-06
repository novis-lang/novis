# Handoff

## State

**Goal 9 stage 2 is green.** `crates/nvs-db/src/schema.rs` holds the closed vocabulary *and* ADR 0145
§ 1's canonical array form: `Node` (an ordered map/list value, because this crate is sans-io and holds
no Novis value), `ScalarType`'s `Display`/`from_spelling` — `int64`, `text(200)`, `decimal(10,2)` —
and `Schema::to_array`/`from_array`, ordered as § 1 requires: declaration order for columns and for a
key's own columns, name order for tables, constraints and indexes. `from_array` builds through the
same builders a program calls, so a file cannot say anything a program could not have built. The five
tests stage 2's check names are all on disk under those names; three were renamed from what the
previous session landed, and nothing else linked the old names.

**`examples/schema.nvs` is on disk and does not compile yet, on purpose.** It is stage 6's target,
written against the surface ADR 0145 states — `Schema::fromArray`, `planAgainst`, `applySafe`,
`Plan\Step::grade()/sql()/isRefused()`, `Plan\Grade` — and `nvs run` reports exactly the `E0405`s for
those members and nothing else. **The driver will name it every session until stage 6 lands; that is
the ordinary state, not a regression** — see the playbook bullet for why writing it was still right and
what it masks (every `cargo-named` check of this goal, since program checks run first).

**Not done, deliberately:** `nvs.toml` owes `examples/schema.nvs` an `[[app]]` entry with
`connect = ["schema"]`, the new `db.schema` grant, and a `[db.schema]` SQLite block. That file is one
snapshot every fixture loads, so a key `nvs_config` does not know yet would break all of them — it goes
in with the capability, at stage 6.

## Next group

**Stage 3, the emitters** — one file set: `crates/nvs-db/src/schema.rs` and a new
`crates/nvs-db/src/ddl.rs` beside it, keyed on `crates/nvs-db/src/sql.rs:72`'s four-valued `Dialect`
(four, not five — ADR 0145 § 8: MariaDB and MySQL share SQL text exactly).

- [ ] **`CREATE TABLE` in all four dialects**, one named test per construct — goal stage 3.2, ADR 0145
      § 8. `every_construct_in_the_vocabulary_emits_in_all_four_dialects` and
      `the_emitters_follow_dialect_rather_than_driver`. Anchors: `crates/nvs-db/src/sql.rs:72`
      (`Dialect`), `crates/nvs-db/src/schema.rs:192` (`ScalarType`, whose `Display` is the *canonical*
      spelling and not the SQL one), `crates/nvs-db/src/schema.rs:300` (`ColumnDefault`),
      `crates/nvs-db/src/schema.rs:483` (`Table`), `crates/nvs-db/src/schema.rs:651` (`Schema`).
- [ ] **MySQL's two departures**: an index is declared inside the `CREATE TABLE` (it has no
      `IF NOT EXISTS` for one), and an indexed text column takes a prefix length.
      `mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists` and
      `mysql_gives_an_indexed_text_column_a_prefix_length`. Anchors:
      `crates/nvs-db/src/schema.rs:461` (`Key`), `crates/nvs-db/src/sql.rs:72`.
- [ ] **A step, its grade and its reason** — ADR 0145 §§ 6 and 8, plus § 7's new paragraph: a *report*
      is a step the plan carries and never applies, and `applySafe` reads only the grades of the steps
      it would run. `every_step_carries_terminated_executable_sql_including_the_ones_apply_refuses` and
      `sqlite_rebuilds_the_table_for_an_alter_it_cannot_express`. Anchors:
      `crates/nvs-db/src/schema.rs:651` (`Schema`), `crates/nvs-db/src/sql.rs:72` (`Dialect`).

## Backlog

- `examples/schema.json` — stage 6's `nvs schema plan --schema examples/schema.json` check wants it; it
  is not in `files`, so it blocks nothing yet. Same array form as the fixture's literal.
- `nvs.toml`'s grant and `[db.schema]` block, with the `db.schema` capability — stage 6, ADR 0145 § 9.
- The fixture's member spelling is a proposal, not a decision: stage 6 either builds it or edits the
  fixture, whose source is not frozen (its four printed lines are).
- ADR 0145 § 11 owes the queue's partial unique index an answer before stage 7 retires
  `crates/nvs-stdlib/src/queue.rs`'s four `MIGRATION_*` lists.
