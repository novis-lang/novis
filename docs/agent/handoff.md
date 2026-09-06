# Handoff

## State

**Goal 9 stage 4's read direction has its SQL.** `crates/nvs-db/src/catalog.rs` is ADR 0145 § 4:
`Read` names the two reads an introspection makes — `Columns` and `Indexes` — `Read::row` names the
columns each answers in order, and `query(read, dialect)` is the eight statements, keyed on `Dialect`
and never on `Driver`. The module's own doc owns every design call: why the primary key arrives
through the index read on all four, why SQL Server's index read is the one statement reaching `sys`,
why an out-of-vocabulary index is dropped rather than reported mangled, and four known gaps.

Slices 1 and 3 of the group both landed and are **one commit**, because they are one file and
`session.py --wrap` stages by path. SQLite is asserted end to end against a real in-memory
`rusqlite` — `crate::ddl` emits, the catalog read reads it back — which is the only dialect whose
server is a file and so the only one testable with no container.

**`examples/schema.nvs` is still the driver's reported failure, and that is the ordinary state**
until stage 6 lands the `Core\Db\Schema` member; the playbook bullet owns the `cargo-named` checks it
masks. `nvs.toml` still owes it an `[[app]]` with `connect = ["schema"]`, the `db.schema` grant and a
`[db.schema]` SQLite block.

Nothing in the orientation pack was missing: `[context]` printed § 4 and the three anchors the item
named, and all three were the ones needed.

## Next group

**Stage 4 slice 2, the row assembly** — one file set: `crates/nvs-db/src/catalog.rs` for the reverse
map and `crates/nvs-db/src/schema.rs` for what it builds. All of it is ADR 0145 § 4, and the finding
that splits it into three is this: **the reverse type map is over the *server's* printed spelling,
not over what `ddl.rs` emitted.** PostgreSQL's `format_type` answers `character varying(200)` where
the emitter wrote `VARCHAR(200)`, and `timestamp with time zone` where it wrote `TIMESTAMPTZ`. It is
also **not injective** — PG and SQLite both read `INTEGER` for `Int(Normal)` and `Uint(Small)`,
`BYTEA`/`BLOB` for both `Bytes` widths, and `NUMERIC(20, 0)` for `Uint(Big)` and for
`Decimal { precision: 20, scale: 0 }` — so the map is a *choice* function and which case it picks is
§ 5's normalisation budget, decided in the doc comment beside it.

- [ ] **The type spelling becomes a `ScalarType`, per dialect** — `every_catalog_spelling_the_emitter_wrote_reads_back_as_its_own_type`.
      The reverse of `column_type`, over the server's own words, lower-cased and whitespace-tolerant
      as `from_spelling` already is. Its test is a round trip over the emitters rather than a table of
      expected names: for every `ScalarType` and every `Dialect`, the emitted spelling must read back
      as a type whose emission is the same string. Anchors:
      `crates/nvs-db/src/ddl.rs:199` (`column_type`), `crates/nvs-db/src/schema.rs:855`
      (`from_spelling`, the canonical-name reader this must not be confused with),
      `crates/nvs-db/src/catalog.rs:141` (`query`, whose `Read::row` doc says which value is the type).
- [ ] **The rows become one `Schema` value** — `every_driver_introspects_into_the_same_schema_value_shape`.
      A driver-neutral row struct this module owns, then a fold over the two reads' rows in their
      `ORDER BY` order: columns into `Column`s in ordinal order, index rows into the primary key
      (`primary = 1`), the unique constraints (`unique = 1`) and the plain indexes. The SQLite
      assertion in `sqlite_reads_sqlite_master_and_its_pragmas` is the fixture to extend — it already
      applies `ddl::create_table` and reads both statements back.
      Anchors: `crates/nvs-db/src/catalog.rs:90` (`Read`, and `Read::row`'s positions),
      `crates/nvs-db/src/schema.rs:489` (`Table`), `crates/nvs-db/src/schema.rs:358` (`Column`).
- [ ] **A server's default spelling becomes a `ColumnDefault`** — `a_default_the_emitter_wrote_reads_back_as_the_same_case`.
      The seven cases plus `Now`, over what each catalog prints: PostgreSQL casts (`'x'::text`),
      MySQL's bare literal and its `CURRENT_TIMESTAMP`, SQL Server's doubled parentheses
      (`(('x'))`), SQLite's verbatim text. A spelling outside the set is refused rather than passed
      through, exactly as `from_spelling` refuses a type.
      Anchors: `crates/nvs-db/src/schema.rs:306` (`ColumnDefault`), `crates/nvs-db/src/ddl.rs:199`.

## Backlog

- Stage 5's normalisation is where every gap in `catalog.rs`'s module doc is paid for — ADR 0145 § 5.
- The four-server matrix has never run a catalog query; `crates/nvs-db/src/matrix.rs` is the harness
  and `NVS_DB_MATRIX_DRIVER` the gate, so the non-SQLite statements are unproven against a server.
- Stage 6's `Core\Db\Schema` member is what turns the driver's reported failure green — ADR 0145 § 9.
- `nvs.toml` owes `examples/schema.nvs` its `[[app]]`, `db.schema` grant and `[db.schema]` block.
