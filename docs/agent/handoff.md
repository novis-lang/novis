# Handoff

## State

**Goal 9 stage 4's read direction is whole but for the assembly.** `crates/nvs-db/src/catalog.rs`
now carries both reverse maps beside § 4's SQL: `scalar_type` reads a server's own type spelling
into a `ScalarType` per dialect, and `column_default` reads a server's own default expression into
a `ColumnDefault`, type-directed. Both are asserted as a **round trip over `crate::ddl`'s
emitters** rather than as a table of expected names, plus one test each for the words a server uses
that no emitter wrote — `character varying(200)`, `((42))`, `'hi'::character varying`, an unquoted
MySQL literal.

The type map is deliberately **not injective**, and its six choices are named case by case in
`scalar_type`'s own doc: each is a normalisation § 5 owes the *declared* side, never something this
module hides. SQLite is the one dialect needing no choice for `uint64`, because the emitter gave it
and `decimal` two different words.

`ScalarType::check`, `ColumnDefault::fits` and `ddl::literal` are `pub(crate)` now, so a value read
out of a catalog passes the same two rules a built one does and an introspection cannot mint a
column the builder would refuse.

**`examples/schema.nvs` is still the driver's reported failure, and that is the ordinary state**
until stage 6 lands the `Core\Db\Schema` member. `nvs.toml` still owes it an `[[app]]` with
`connect = ["schema"]`, the `db.schema` grant and a `[db.schema]` SQLite block.

Nothing in the orientation pack was missing.

## Next group

**Stage 4's last slice and its proof** — one file set: `crates/nvs-db/src/catalog.rs` for the
assembly and `crates/nvs-db/src/schema.rs` for the builders it calls. The finding that shapes the
first item: **`nvs-db` has no unified row value.** `crates/nvs-db/src/lib.rs:177` exports `PgRow`,
`MySqlRow` and `SqliteValue` as three per-driver types and nothing common, so the assembly cannot
take "a row" — it takes a neutral struct per read, filled by `nvs-stdlib` from whichever driver
answered, which is also what keeps this crate sans-io per ADR 0132.

- [ ] **The rows become one `Schema` value** — `every_driver_introspects_into_the_same_schema_value_shape`.
      Two neutral row structs mirroring `Read::row`'s positions, grouped by table and then by index,
      through the two readers this session landed. The primary key arrives on the *index* rows, so a
      column is built before any key is added to it. Anchors:
      `crates/nvs-db/src/catalog.rs:117` (`Read::row`, the positional contract the structs mirror),
      `crates/nvs-db/src/catalog.rs:435` (`scalar_type`), `crates/nvs-db/src/catalog.rs:661`
      (`column_default`), `crates/nvs-db/src/schema.rs:512` (`Table::new`, then `primary_key`,
      `unique`, `index` at 546, 563, 577), `crates/nvs-db/src/schema.rs:679` (`Schema::new`).
- [ ] **SQLite proves the assembly against a real server** — `a_schema_applied_to_sqlite_assembles_back_into_itself`.
      `crate::ddl` emits, `rusqlite` applies, the two catalog reads come back, the assembly runs, and
      the value is compared to the one that went in. **§ 5's diff does not exist yet** — `plan.rs`
      has `Plan::new` and no `between`, so the assertion is value equality and what is *not* equal is
      the list stage 5 inherits. Anchors: `crates/nvs-db/src/catalog.rs:934`
      (`sqlite_reads_sqlite_master_and_its_pragmas`, the in-memory fixture to extend),
      `crates/nvs-db/src/ddl.rs:103` (`create_schema`), `crates/nvs-db/src/plan.rs:293` (`Plan`).

## Backlog

- Stage 6's `Core\Db\Schema` member and its `nvs.toml` block — `docs/agent/loop-goal.toml:4597`.
- § 5's normalisation must widen the declared side the way `scalar_type`'s doc names —
  `crates/nvs-db/src/catalog.rs:435`, ADR 0145 § 5.
- SQL Server's `DATETIME_PRECISION` is still not folded into its type spelling — gap 2 of
  `crates/nvs-db/src/catalog.rs`'s module doc.
- An index outside § 11's vocabulary is dropped rather than reported — gap 1 of the same doc.
