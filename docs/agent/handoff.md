# Handoff

## State

**`queryAs<T>`'s row is on `Core\Db\Queryable`'s two classes, and its body is owed.** The member is
generic (`CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Written("T")])`), so `$db->queryAs<Person>(…)` type-
checks as `Core\Db\Rows<Person>`, a call naming no type argument is `E0442` and one naming a scalar is
`E0465`. The helper faults naming `nvs_stdlib::db`'s **known gap 9**, which is the one home for what is
left: the walk over `ClassDesc::db_codec` that ADR 0071 § 5 specifies.

**An instance member can now take the class written at its call site.** `WRITTEN_CLASS_MEMBERS` gained
both `Core\Db\Connection::queryAs` and `Core\Db\Transaction::queryAs` — keyed by the *declaring* class,
so ADR 0043's delegation is two rows — and the pair goes ahead of **everything**, receiver included:
`args: [5]` is descriptor, list flag, receiver, `$sql`, `$params`. That roster's doc comment is the only
home for the ordering; `nvs_types::expr::calls::infer_method_call` records the class and
`nvs_ir::lower::expr::lower_method_call` emits the two constants before the receiver is opened.

**`examples/db.nvs` now compiles and stops at run time instead**, on gap 9's fault. The driver's
acceptance line still names `examples/queue.nvs`, which is Stage 8's unlanded queue and not a regression
(`Core\Queue` has never existed; ADR 0084 is what lands it). **The CA is still not in git**;
`nvs_host::tls`'s module doc owns why.

## Next group

**The hydration body, in three slices — the file set is `crates/nvs-stdlib/src/db.rs` alone, with
`examples/db.nvs` as the proof.**

- [ ] **A `Rows` carries the class its rows hydrate into.** `ROWS` is `crates/nvs-stdlib/src/db.rs:646`
      with one slot; add a second holding the descriptor or `null`, and write it from both producers —
      `crates/nvs-stdlib/src/db.rs:2394` (`query`, which writes `null`) and
      `crates/nvs-stdlib/src/db.rs:2432` (`queryAs`, which writes `args[0]`). Lazy rather than eager so
      `value()`, `column()` and `count()` keep reading the columns they read today. ADR 0067 § 4, spec
      § 18's *Results* table.
- [ ] **One row into one `T`.** `crates/nvs-stdlib/src/db.rs:2860` (`rows_all`),
      `crates/nvs-stdlib/src/db.rs:2875` (`rows_iterate`) and `crates/nvs-stdlib/src/db.rs:2911`
      (`rows_first`) each build a `ROW`; where the new slot holds a descriptor they build the class
      instead. The walk is `ClassDesc::db_codec()` + `db_codec_class(i)` + `nvs_runtime::construct`, and
      `crates/nvs-stdlib/src/json.rs:1167`'s `decode_fields` is the shape to follow — ADR 0071 § 5's
      accumulate-then-construct, with the row's *already typed* column values in place of JSON scalars,
      so each field is a `CodecTy` check and not a parse. The refusals belong on `QUERY_AS`
      (`crates/nvs-stdlib/src/db.rs:1614`) so they open on a hole, per the playbook bullet.
- [ ] **`examples/db.nvs` end to end**, `crates/nvs-stdlib/src/db.rs:118`'s gap 9 deleted, and a fourth
      `.nvst` case if the hydration has a compile-time boundary worth pinning (a `T` carrying no
      `#[Db\Derive]` codec is a *runtime* refusal today — gap 9 says why, and both type diagnostic bands
      are full).

## Backlog

- `stream`/`streamAs` and `close`, plus § 18's three readonly properties — `db.rs` gap 5.
- `Rows::columns()` needs a `Core\Db\Column` and a `ColumnType` enum — `db.rs` gap 5.
- § 9's five structured columns do not read back — `db.rs` gap 6.
- `{timeout?: Duration}` is in no statement row — `db.rs` gap 7.
- ADR 0067 § 13's per-core pool — the plan's Stages 3 to 7.
- `Db\DbError` is not in spec § 10's tree — `db.rs` gap 4.
