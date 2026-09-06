# Handoff

## State

**Goal 9 stage 5's diff is whole.** `nvs_db::plan::diff` (`crates/nvs-db/src/plan.rs:418`) walks two
`Schema` values, matches tables and columns by name and never by position, and hands every `Change`
to `crate::ddl::step` for the SQL and the grade. § 5's three normalisations are `same_key` and
`same_column` (`crates/nvs-db/src/plan.rs:532`, `:504`) — a unique constraint is its columns, a plain
index is its name and its columns, and SQLite's rowid loses the declared integer width. They live in
the *comparison* and not in the value on purpose, and `diff`'s own doc owns the reason.

**§ 5's acceptance property holds where a server is a file**:
`an_applied_schema_introspects_back_to_an_empty_plan_on_sqlite`. **§ 6's eight grade tests are on
disk**, so stage 5's `nvs-db (the grades)` check is green and `nvs-db (apply, introspect, empty)`
waits only on Docker. The driver's reported failure is stage 6's `examples/schema.nvs`, which cannot
pass before the member exists. Nothing in the orientation pack was missing.

## Next group

**Stage 6's member** — one file set: `crates/nvs-stdlib/src/db/` for the class, with
`crates/nvs-stdlib/src/registry.rs` and `crates/nvs-config/src/capability.rs` beside it. The finding
that shapes the group: everything below `Core\Db\Schema` is on disk and proved — the vocabulary, the
emitters, the readers, the diff and the grades — so stage 6 is a surface over `nvs-db` that decides
nothing, and the four slices are in dependency order.

- [ ] **`db.schema` joins the capability roster** — ADR 0145 § 9: it names connection blocks like
      `db.connect` and gates whether DDL may be issued at all rather than what may be reached, and an
      ungranted name throws naming the capability. Anchors:
      `crates/nvs-config/src/capability.rs:76` (`DbConnect`, the arm it sits beside),
      `crates/nvs-config/src/capability.rs:201` (where a capability is spelled).
- [ ] **`Core\Db\Schema::fromArray` and `toArray`** — ADR 0145 § 1's canonical array form, over
      `nvs_db::schema::Node`, which exists for exactly this and is the only converter. The five edits
      of `docs/agent/conventions.md` § *A `Core` member*. Anchors: `crates/nvs-db/src/schema.rs:731`
      (`Node`), `crates/nvs-stdlib/src/db/registry.rs:134` (`Core\Db\Row`'s `CoreClass`, the shape to
      copy), `crates/nvs-stdlib/src/registry.rs:1255` (`CLASSES`, where the class is listed).
- [ ] **`planAgainst`, `applySafe` and `applyIncludingRisky`** — ADR 0145 § 9, with
      `plan_against_needs_only_the_db_connect_a_program_already_holds`,
      `applying_without_the_db_schema_capability_throws_naming_it` and
      `apply_safe_refuses_a_plan_holding_a_step_that_is_not_safe`. Anchors:
      `crates/nvs-db/src/plan.rs:418` (`diff`, which `planAgainst` is a connection plus a call to),
      `crates/nvs-db/src/plan.rs:350` (`Plan::first_refused`, which is `applySafe`'s whole refusal),
      `crates/nvs-stdlib/src/db/pool.rs:269` (`filed_connection`, how a member reaches the driver).
- [ ] **`nvs.toml` owes the example an app** — an `[[app]]` for `examples/schema.nvs` with
      `connect = ["schema"]`, the `db.schema` grant and a `[db.schema]` SQLite block; that example is
      the driver's reported failure. The file is the repository's own root `nvs.toml`, whose first
      `[[app]]` is at line 23. Anchors: `docs/agent/loop-goal.toml:4597` (the four lines the example
      must print), `crates/nvs-config/src/capability.rs:201` (the grant name the block writes).

## Backlog

- The four `an_applied_schema_introspects_back_to_an_empty_plan_on_*` that need Docker —
  `docs/agent/loop-goal.toml:4544`.
- `nvs schema plan|apply|dump` is stage 6's CLI, and stage 4's second check waits on it —
  `docs/agent/loop-goal.toml:4533`.
- A primary key that differs between two tables *both* sides already have produces no step, because
  `Change` has no case for one — `crates/nvs-db/src/plan.rs:418`'s doc owns the reasoning.
- `unquote`'s unquoted fallback is MySQL's alone; on the other three an unquoted spelling is an
  expression and is read as text — gap 4 of `crates/nvs-db/src/catalog.rs`'s module doc.
