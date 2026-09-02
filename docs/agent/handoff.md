# Handoff

## State

**`Rows::columns()`'s gap 5 has lost its driver half, and its enum is decided rather than invented.**
Spec § 18 declares `ColumnType`'s fourteen cases at `docs/spec/01-core-library.md:1223`, in the same
fenced block as `Isolation` — so the enum is the spec's, **not** ADR 0067 § 9's type map read as an
enum, and `crates/nvs-stdlib/src/db.rs`'s `ROWS` doc now records the three places reading § 9 that way
would have gone wrong: `JSON` and `TEXT` both decode to `tainted string`, an array decodes to
`array<T>` with no case to describe it, and § 9's last row is `Other`.

**`nvs_db::ColumnType` and `PgColumn::column_type` land** — the enum beside `Isolation` in
`crates/nvs-db/src/conn.rs`, the classification in `pg.rs` reading the OID plus § 9's `BIT(1)`
modifier and nothing else, so a column whose every row is NULL still has a type. `mod oid` gained the
text family and the two JSON OIDs, which its own doc explains: the table now answers *two* questions,
and `varchar` is `Text` where `inet` is `Other` though both decode alike. A PostgreSQL `ENUM` answers
`Other`, because an enum type's OID is not bootstrap data; `column_type`'s doc owns that limit.

**What is left of gap 5 is two `Core` types and a slot.** `ROWS`' slot holds decoded rows and nothing
else, so `nvs_db::PgRows::columns` has to be captured where the receiver is built or not at all.

**The pack did not print spec § 18's enum fence** — the item's anchors reached line 1209 and the
declaration is at 1223. A `docs/spec/01-core-library.md:"### Enums, settings and errors"` selector
belongs in `[context]`, and `nvs-runtime/src/object.rs` is still missing from `[context] modules`.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded
`Core\Queue` (ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s module
doc owns why.

## Next group

**`columns()` in three slices, and the file set is `crates/nvs-stdlib/src/db.rs` and
`crates/nvs-stdlib/src/registry.rs`.** Nothing in `nvs-db` moves again.

- [ ] **`Core\Db\ColumnType` registers, as `ISOLATION`'s twin.** A `CoreEnum` plus its `EnumDoc`
      beside `crates/nvs-stdlib/src/db.rs:566` and `crates/nvs-stdlib/src/db.rs:585`, and the roster
      row beside `crates/nvs-stdlib/src/registry.rs:1870`. The cases, their order and their
      ordinals are the spec's at `docs/spec/01-core-library.md:1223`; the wire half to keep it
      honest against is `crates/nvs-db/src/conn.rs:220`, whose doc is where the value/description
      split is written down. ADR 0067 § 9, spec § 18.
- [ ] **`Core\Db\Column` registers, and `ROWS`' slot starts carrying the descriptions.** Three
      readers per `docs/spec/01-core-library.md:1200`, a class row beside
      `crates/nvs-stdlib/src/registry.rs:1413`, and the capture at the two places the receiver is
      built — `crates/nvs-stdlib/src/db.rs:2459` and `crates/nvs-stdlib/src/db.rs:2662` — off
      `crates/nvs-db/src/pg.rs:2445`. **`nullable()` has no source**: a PostgreSQL `RowDescription`
      carries no NOT NULL flag and the catalog lookup that would is what ADR 0067 § 9's table is
      written to avoid, so decide it in the slice and say so on the member — `true` is the safe
      total answer.
- [ ] **`ROWS` gains its sixth member.** The five edits of *A `Core` member* against
      `crates/nvs-stdlib/src/db.rs:691`, answering `array<Column>` off the slot the previous slice
      filled, plus three `.nvst` cases — one of them a column whose every row is NULL, which is the
      claim `columns()` exists for. ADR 0063, spec § 18.

## Backlog

- `stream`/`streamAs`, `close` and § 18's three readonly properties on `Connection` — `db.rs` gap 5.
- `Db\DbError` is not in spec § 10's tree, so a refusal carries no `kind` — `db.rs` gap 4.
- Neither `query` nor `execute` declares `{timeout?: Duration}` — `db.rs` gap 6.
- `queryAs<T>`'s three run-time refusals that belong at compile time — `db.rs` gap 7.
- ADR 0067 § 13's per-core pool, Stages 3-7 of the goal.
- `open` waits on a shape-parameter type — the plan's *Open now*.
