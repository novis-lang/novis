# Handoff

## State

**ADR 0067 § 9's five structured columns now reach a `#[Db\Derive]` field, so gap 6 is closed.**
Hydration's `CodecTy::Class` arm checks the instance the driver's components built against the
class the field declared, by rendered name: `converted` takes the label both of its callers already
hold (`nvs_runtime::CodecField::class` — the field's own for a scalar field, the element's for a
list's), and ADR 0051's reservation of the `Core\` prefix for Tier 0 is why comparing names is an
identity test. Stage 0 never refused such a field — `nvs_types::derive`'s `DB_COLUMN_CLASSES`
lists all five — so the whole of the gap was that one arm. `examples/db.nvs`'s `Day` class reads a
`DATE` back and prints it; a `TIMESTAMPTZ` into that same field is a `ParseError` naming the column.

**`db.rs`'s known-gap list is renumbered**: the old 6 is gone, so 7→6, 8→7 and 9→8. Every reference
inside the file moved with it, and nothing outside the file names one.

**`docs/agent/goals/5-database.toml` had drifted from `loop-goal.toml`** — it still named
`crates/nvs-host/src/stream.rs`, which does not exist, and lacked the `[docker.copy]` CA step — so
it was copied over whole. The playbook's rule is that the two are byte-identical by construction,
and the next `goal-switch.py` would otherwise have reverted both fixes.

**The orientation pack has no `nvs-runtime` in `[context] modules`**, and this group needed
`crates/nvs-runtime/src/object.rs` for `CodecField` and `NvsObj::class_of`. A `nvs-runtime/src/object.rs`
pattern belongs in that field.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded
`Core\Queue` (ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s
module doc owns why.

## Next group

**`Rows::columns()` — gap 5, in three slices. The file set is `crates/nvs-stdlib/src/db.rs` and
`crates/nvs-db/src/pg.rs`, with `docs/spec/01-core-library.md:1197` read first.** The first decides
what the other two are building against.

- [ ] **Find where `Column` and `ColumnType`'s cases are actually written down.**
      `docs/spec/01-core-library.md:1197` is the only line in the spec naming
      `->columns(): array<Column>`, and the spec holds no `ColumnType` at all — so the fourteen
      cases `crates/nvs-stdlib/src/db.rs:651`'s gap 5 promises are ADR 0067 § 9's type map read as
      an enum, or they are nowhere and this slice decides them. Say which in the gap.
      ADR 0067 § 9, § 18.
- [ ] **`nvs-db` answers a column's *declared* type, not just its value.**
      `crates/nvs-db/src/pg.rs:1110`'s `type_oid` is on `PgColumn` and
      `crates/nvs-db/src/pg.rs:1675`'s `scalar` maps an OID to a value — the other question, since
      a NULL column still has a declared type. The classification goes beside it, in `nvs-db`,
      because the OID table is that crate's. ADR 0067 § 9.
- [ ] **`Core\ColumnType` and `Core\Db\Column` register, and `ROWS` gains its sixth member.**
      `crates/nvs-stdlib/src/db.rs:585`'s `ISOLATION` is the `CoreEnum` shape to copy,
      `crates/nvs-stdlib/src/db.rs:666`'s `ROWS` is where the row goes, and closing this rewrites
      gap 5 at `crates/nvs-stdlib/src/db.rs:651`. ADR 0067 § 18.

## Backlog

- `queryAs<T>`'s three refusals are per row and not at compile time — `db.rs` gap 8.
- `Db\DbError` is not in spec § 10's tree, so a server refusal is a plain `RuntimeError` — gap 4.
- `Db::open(Settings)` waits on a shape-parameter type — the plan's *Open now*.
- `stream`, `streamAs`, `close` and § 18's three readonly properties are owed whole — `db.rs` gap 5.
- § 13's per-core pool and its reset are Stages 3 to 7 — `docs/plan/m8.md`.
- Stage 8's `Core\Queue` (ADR 0084) is what the driver's failing acceptance line names.
