# Handoff

## State

**`queryAs<T>` hydrates, and `examples/db.nvs` runs end to end** — its six frozen lines, `typed row
ok` included. A `Core\Db\Rows` carries a second slot holding the class its call site wrote (`null` from
`query`), read only by the three members that hand a row out, so `value()`, `column()` and `count()`
cost what they always did. `db.rs`'s `hydrate` is ADR 0071 § 5's accumulate-then-construct over
`ClassDesc::db_codec`, and every field is a *check* rather than a parse: § 9's map already decoded the
column, so what is left is whether that value is the one the field declares, with § 6 deciding `int`
against `uint`.

**Gap 9 is now only about where the "no" is said** — `db.rs`'s module doc owns the list. Three refusals
are per row that belong at compile time (a `T` with no `#[Db\Derive]`, a `queryAs<array<C>>`, a field
the derive pass erased to `CodecTy::Opaque`), because both type diagnostic bands are full. The
issue-carrying throw is a `ParseError` and not § 8's `DbError`, which gap 4 owns.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded queue
and not a regression — `Core\Queue` has never existed, and ADR 0084 is what lands it. **The CA is
still not in git**; `nvs_host::tls`'s module doc owns why.

## Next group

**ADR 0067 § 9's five structured columns, in three slices — the file set is
`crates/nvs-stdlib/src/db.rs`, with `crates/nvs-db/src/pg.rs` read only and `crates/nvs-stdlib/src/time.rs`
for the builders.** This is `db.rs`'s known gap 6, and it is what four landed `Core\Db\Row` readers and
the hydration's own class arm are all waiting on.

- [ ] **A `DATE`, `TIME`, `TIMESTAMP`, `TIMESTAMPTZ` or `UUID` column decodes.**
      `crates/nvs-stdlib/src/db.rs:2489` is where `PgColumn::decode`'s `None` becomes the
      `crates/nvs-stdlib/src/db.rs:2118` refusal; ask `crates/nvs-db/src/pg.rs:1660`'s
      `PgColumn::scalar` for the parsed components instead and build the instance here, which is the
      only crate that can. `crates/nvs-stdlib/src/time.rs:3000` is the builder shape and
      `crates/nvs-stdlib/src/uuid.rs:117` the other class. ADR 0067 § 9.
- [ ] **`Core\Db\Row`'s four typed readers answer one.** `crates/nvs-stdlib/src/db.rs:3662`
      (`instant`), `crates/nvs-stdlib/src/db.rs:3682` (`date`) and `crates/nvs-stdlib/src/db.rs:3717`
      (`uuid`), plus `time` beside them: each has been waiting for a column it could never see, so the
      question is only which tag they now accept. ADR 0067 § 6.
- [ ] **The hydration's class arm, and gap 6 rewritten.** `crates/nvs-stdlib/src/db.rs:3081`'s
      `converted` refuses `CodecTy::Class` today; a field declaring one of § 9's five classes should
      take the instance the column now decodes to, checked against `ClassDesc::db_codec_class(index)`
      — which means `hydrate` (`crates/nvs-stdlib/src/db.rs:2930`) has to pass the field's index
      through. Then `crates/nvs-stdlib/src/db.rs:103`'s gap 6 says what is left, if anything.

## Backlog

- `Rows::columns()` — `db.rs` gap 5: needs `Core\Db\Column`, a `Core\ColumnType` enum and an OID
  classification `nvs-db` does not expose.
- `stream`/`streamAs`, `close` and § 18's three readonly `Connection` properties — `db.rs` gap 5.
- The per-core pool and its reset — ADR 0067 § 13, the plan's Stages 3 to 7.
- `Core\Queue` — ADR 0084, Stage 8; this is what the driver's failing acceptance line names.
- `Db\DbError` in spec § 10's tree — `db.rs` gap 4, and it is the class ADR 0071 § 5 gives `issues` to
  for the `Db` half.
- `decimal` and `bytes` fields erase to `CodecTy::Opaque` — `nvs_types::derive`'s own gap 1, which is
  why a `#[Db\Derive]` class cannot declare one yet.
