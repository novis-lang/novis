# Handoff

## State

**ADR 0067 § 9's five structured columns decode, end to end.** A `DATE`, `TIME`, `TIMESTAMP`,
`TIMESTAMPTZ` or `UUID` reads back as the `Core\Time`/`Core\Uuid` instance the map names, and
`Core\Db\Row`'s `date`, `time`, `instant` and `uuid` answer off it — `examples/db.nvs` prints all
four against the compose PostgreSQL and its `want` list in `docs/agent/loop-goal.toml` (and the
`docs/agent/goals/5-database.toml` copy) is the ten lines it now prints.

**The split is the one `nvs_db::PgDate`'s doc argues.** `nvs-db` parses and hands components over
(`PgColumn::scalar`); `nvs-stdlib` allocates, because the instance is a class it declares:
`crate::time`'s `date_at`/`time_of_day_at`/`datetime_at`/`instant_at` seams over a `Civil`,
`crate::uuid::of_octets`, joined by `db.rs`'s `column_value` — which is also the recursive half, so
an `array<Core\Uuid>` needs nothing of its own. Every range check is in `time.rs`: PostgreSQL's
`24:00:00` and a year past the calendar are `None` there and a thrown refusal naming the column
here. **A `PgConn` now holds its declared zone** (`PgConn::time_zone()`), because § 9's zone-less
`TIMESTAMP` is read in it and the target does not outlive the handshake.

**Gap 6 is now only the hydration's `CodecTy::Class` arm** — `db.rs`'s module doc owns the list. A
`#[Db\Derive]` field declared as one of those five classes still refuses, because that arm has
nothing to check the built instance against. § 6's roster has no `dateTime` reader, so a zone-less
`TIMESTAMP` is reached through `get`, and that is the ADR's decision rather than a gap.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded
queue and not a regression — ADR 0084 is what lands it. **The CA is still not in git**;
`nvs_host::tls`'s module doc owns why.

## Next group

**Gap 6's last arm, in three slices — the file set is `crates/nvs-stdlib/src/db.rs`, with
`crates/nvs-types/src/derive.rs` and `crates/nvs-runtime/src/object.rs` read first and
`examples/db.nvs` last.** Do them in this order: the first decides whether the other two have
anything to do.

- [ ] **Find out what a `#[Db\Derive]` field declared `Core\Time\Date` survives.**
      `crates/nvs-types/src/derive.rs:435` maps a `Ty::Class` to `CodecTy::Class` and keeps the
      label, but § 9's type-map check (`E0756`, Stage 0) may refuse the field before hydration ever
      sees it, and the derive pass may erase it to `CodecTy::Opaque` instead. One `nvs run` over a
      three-line class answers all of it, and it decides the shape of the next slice. ADR 0067 § 9,
      ADR 0071 § 5.
- [ ] **The hydration's `CodecTy::Class` arm checks the instance against the field's class.**
      `crates/nvs-stdlib/src/db.rs:3081` is the hydration and the arm is its `CodecTy::Class` row.
      `crates/nvs-runtime/src/object.rs:310`'s `codec_classes` is the resolved descriptor for
      `#[Json\Derive]`'s nested classes and is filled only for a class *of the unit's own making*,
      so it is null for a `Core` one; check whether the row half has a twin at all, and otherwise
      match `CodecField::class`'s label (`crates/nvs-runtime/src/object.rs:677`) against
      `crate::time::DATE`/`TIME_OF_DAY`/`DATETIME`/`INSTANT` and `crate::uuid::NAME` through
      `crate::instance::is_instance`. Then rewrite gap 6 at `crates/nvs-stdlib/src/db.rs:103`.
- [ ] **`examples/db.nvs` grows a class with a `Core\Time\Date` field**, and both `want` lists gain
      its line — `docs/agent/loop-goal.toml:2963` and `docs/agent/goals/5-database.toml:2953`, which
      are one check in two files. `examples/db.nvs:100` is where the structured block sits.

## Backlog

- `Rows::columns()` needs a `Core\Db\Column` class and a `Core\ColumnType` enum — `db.rs` gap 5.
- `queryAs<T>`'s three refusals are per row and not at compile time — `db.rs` gap 9.
- `Db\DbError` is not in spec § 10's tree, so a server refusal is a plain `RuntimeError` — gap 4.
- `Db::open(Settings)` waits on a shape-parameter type — the plan's *Open now*.
- § 13's per-core pool and its reset are Stages 3 to 7 — `docs/plan/m8.md`.
- Stage 8's `Core\Queue` (ADR 0084) is what the driver's failing acceptance line names.
