# Handoff

## State

**Goal 9 stage 3 is complete** — all six of its `cargo-named` checks are green.
`crates/nvs-db/src/plan.rs` is ADR 0145 §§ 6-8's vocabulary and holds no SQL: `Grade` (ordered, so
§ 6's grade-up rule is `Grade::up_to`), `Change` (the closed set a diff may produce), `Step` and
`Plan`. `crates/nvs-db/src/ddl.rs:495`'s `step` is the only place a `Step` is built, because § 6 puts
the grade in the dialect emitter. Both modules' own doc comments own every design call they made —
the after-shape convention on `Change`, why `Plan::first_refused` reads only the runnable steps, and
the six known gaps that are § 5's input rather than a bug.

**`examples/schema.nvs` is still the driver's reported failure, and that is the ordinary state** until
stage 6 lands the `Core\Db\Schema` member — see the playbook bullet for the `cargo-named` checks it
masks. `nvs.toml` still owes it an `[[app]]` with `connect = ["schema"]`, the `db.schema` grant and a
`[db.schema]` SQLite block.

**The `[context]` manifest gap is closed in the tree, not just reported**: `adrs` now names
`0145 §4`, `0145 §5` and `0145 §9` — the three sections stages 4, 5 and 6 need — and `modules` gained
`ddl.rs` and `plan.rs`. §§ 6-8 are deliberately *not* listed: they are landed, and their rules now
live in `plan.rs`'s module doc, which the map prints.

## Next group

**Stage 4, the catalog readers** — one file set: a new `crates/nvs-db/src/catalog.rs` beside the
emitters, and `crates/nvs-db/src/schema.rs` for the read direction. All three slices are ADR 0145 § 4,
which the pack now prints. Nothing here needs a server: the queries are text and the assembly is over
rows, so both halves are unit-testable exactly as `ddl.rs` is.

- [ ] **The catalog query per dialect** — `postgres_reads_pg_catalog_and_the_others_read_information_schema`.
      One function returning the SQL that reads a database's tables, columns and indexes, keyed on
      `Dialect` and never on `Driver`, as the emitters already are. Anchors:
      `crates/nvs-db/src/sql.rs:72` (`Dialect`), `crates/nvs-db/src/ddl.rs:199` (`column_type`, the
      write direction these have to read back), `crates/nvs-db/src/schema.rs:851`
      (`ScalarType::from_spelling`).
- [ ] **The rows become one `Schema` value** — `every_driver_introspects_into_the_same_schema_value_shape`.
      An *agreement* test over all five: the same catalog rows assemble into the same value whichever
      driver produced them. Anchors: `crates/nvs-db/src/schema.rs:253` (`ScalarType::describes`),
      `crates/nvs-db/src/schema.rs:851`, `crates/nvs-db/src/ddl.rs:199`.
- [ ] **SQLite reads neither catalog** — `sqlite_reads_sqlite_master_and_its_pragmas`. `sqlite_master`
      plus `pragma table_info` and `pragma index_list`, which answer in columns nothing else does.
      Anchors: `crates/nvs-db/src/sqlite.rs:423` (`column_type`), `crates/nvs-db/src/schema.rs:851`.

## Backlog

- `nvs schema dump --connection main` must print `nvs_jobs` and `nvs_dead_jobs` — stage 4's second
  check, `docs/agent/loop-goal.toml:4529`; `crates/nvs-cli/src/queue.rs` is where it is wired in.
- Stage 5's diff over normalized values, and the empty-plan property — ADR 0145 § 5.
- Stage 6: the `Core\Db\Schema` rows, the `db.schema` capability, and `nvs.toml`'s `[[app]]` entry —
  ADR 0145 § 9, and the driver's standing failure.
- A SQL Server default is a separately named constraint and no `Change` carries the name, so a
  default change there is not emitted — `ddl.rs`'s known gap 4, and stage 4 is what can supply it.
- A SQLite unique constraint added after the fact is an index, where one written into a `CREATE TABLE`
  leaves an `sqlite_autoindex_…` — `ddl.rs`'s known gap 5, and § 5's input.
