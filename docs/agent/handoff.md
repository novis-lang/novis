# Handoff

## State

**§ 18's Results surface is on disk: `Core\Db\Rows` has five of its six members and
`Core\Db\Row` has all fourteen** — `crates/nvs-stdlib/src/db.rs:304` and `:392`. `Rows`
reads the one slot `query` filled and never decodes a second time; a `Row` is one object over
*that row's own array* under a second reference, so `all()` over a thousand rows allocates a
thousand objects and no second thousand arrays. `Rows` has left `registry.rs`'s `HANDLES`.

**`columns(): array<Column>` is the one member owed**, and it is a slice rather than a body:
it needs a `Core\Db\Column`, a `Core\ColumnType` enum for spec § 18's fourteen cases, a second
`ROWS` slot for the description, and a classification of a `PgColumn`'s type OID that `nvs-db`
does not expose — `PgColumn::decode` maps an OID to a *value*, which is a different question.
`crates/nvs-stdlib/src/db.rs`'s known gap 5 is that list; gap 5 also owns why `Rows` is not
`Iterable<Row>` (no registry row spells an iterable return).

**The eleven typed readers convert losslessly or throw, and the rule is one sentence**: a reader
answers its own tag, and `int`/`uint` are the single crossing. `instant`, `date`, `time` and
`uuid` are written as the lookups they will always be, and refuse everything today because
`structured_column` (`crates/nvs-stdlib/src/db.rs:1364`) still throws before such a column
reaches a slot — the next group's second item.

**The acceptance check moved one member further**: `examples/transaction.nvs` no longer stops at
`->first()` or `->int()`; it stops at `->execute`, which is the next group's first item.

## Next group

**The write side and the two columns questions — the file set is `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-db/src/pg.rs`, `crates/nvs-stdlib/src/time.rs` and `crates/nvs-stdlib/src/uuid.rs`.**

- [ ] **§ 4's `execute` and the `Db\Write` it answers** — `docs/spec/01-core-library.md:1158` for
      the row, `:1193` for `Write`'s three fields, ADR 0067 § 4. The member joins `query` on
      `crates/nvs-stdlib/src/db.rs:260`'s instance roster and a `WRITE` class goes beside `ROW` at
      `crates/nvs-stdlib/src/db.rs:392`; `Write`'s `affected`/`changed`/`lastId` are **readers and
      not properties** (a `Core` instance has no property a program can reach), filled from
      `crates/nvs-db/src/pg.rs:2379`'s `affected` and `:2402`'s `last_id`. This is what the failing
      acceptance check reaches next.
- [ ] **§ 9's five structured columns, which is what `date`/`time`/`instant`/`uuid` need** — ADR
      0067 § 9. `crates/nvs-db/src/pg.rs:1322`'s `PgScalar` already hands back the parsed
      components; what is missing is building a `Core\Time\Instant`/`Date`/`TimeOfDay` and a
      `Core\Uuid` from them in `nvs-stdlib` and deleting `crates/nvs-stdlib/src/db.rs:1364`'s
      refusal. The instance builders are in `crates/nvs-stdlib/src/time.rs:1123`, `:2445`, `:2741`
      and `crates/nvs-stdlib/src/uuid.rs:117`.
- [ ] **`Rows::columns()` and the `Column`/`ColumnType` it needs** —
      `docs/spec/01-core-library.md:1194` for `Column`, `:1213` for the enum's fourteen cases.
      `crates/nvs-stdlib/src/db.rs:304`'s `ROWS` gains a second slot that
      `crates/nvs-stdlib/src/db.rs:1075`'s `query` fills from `crates/nvs-db/src/pg.rs:1069`'s
      `PgColumn`, which needs a public OID classification there. PostgreSQL's `RowDescription`
      carries **no** nullability, so `Column::nullable` has to decide what it says on a driver that
      cannot know — record it rather than guessing twice.

## Backlog

- `stream`/`streamAs`, `queryAs` and `transaction` — `crates/nvs-stdlib/src/db.rs` known gap 5.
- `open` waits on a shape-*parameter* `CoreTy` — that module's known gap 1, a language-surface call.
- ADR 0067 § 13's per-core pool, Stages 3 to 7 — `docs/plan/m8.md`.
- `Db\DbError`/`Db\RolledBack` are not in spec § 10's tree — known gap 4.
- The four drivers past PostgreSQL — known gap 2, and ADR 0132 has the shape.
- `query` declares no `{timeout?: Duration}` — known gap 7, blocked on a statement-path deadline.
