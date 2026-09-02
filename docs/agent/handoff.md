# Handoff

## State

**`examples/db.nvs` compiles as far as `queryAs<T>`, which is the one thing it still needs**, and
the runtime half that member will hydrate from now exists: ADR 0071's **row** codec travels
`nvs_types::ExprTypeTable::db_codec` → `nvs_ir::ir::Class::db_codec` →
`nvs_runtime::ClassDesc::db_codec()`, beside the JSON one rather than merged with it, because
`#[Json\Field(name:)]` and `#[Db\Field(name:)]` are both legal on one property. `ctor_arity` is
written by whichever format the class declares. `crates/nvs-ir/tests/derived_codecs.rs` pins all
three cases.

**Nothing reads `db_codec()` yet** — it is the input the `queryAs<T>` hydration body was missing,
and the next group spends it.

**`queryAs<T>`'s type half is the unresolved design call, and it is bigger than the item read.**
Spec § 18 writes `Db\Rows<T>`, a *generic* `Core` class: `->all(): array<T>`, `->first(): ?T`,
`Iterable<T>`. `nvs_stdlib::registry::GENERIC_CLASSES` already carries four such classes and
`nvs_types::core_lib` interns `CoreTy::Instance` of one **at its own type variables**, so `Rows`
joining that roster is the shape — but then `query`'s return is `Rows<Row>`, an instance at a
*concrete* argument, and no `CoreTy` variant can spell one. That variant is the next group's first
item.

**The driver's acceptance line names `examples/queue.nvs`, and that is Stage 8's unlanded queue,
not a regression** — `Core\Queue` has never existed, it sits one program leg ahead of
`examples/db.nvs`, and everything behind it stays unchecked until ADR 0084 lands. The playbook
bullet owns the reading.

**The CA is still not in git** — it belongs to the `certs` volume, and `nvs_host::tls`'s module doc
owns why a checkout that has never brought the fixtures up cannot boot from the repo root.

## Next group

**`queryAs<T>`, in three slices — the file set is `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/db.rs` and `crates/nvs-types/src/core_lib.rs`, with `examples/db.nvs` as the
proof.**

- [ ] **`Core\Db\Rows` becomes generic at `T`, and a row can write an instance at concrete
      arguments.** Add `(ROWS_NAME, &["T"])` at `crates/nvs-stdlib/src/registry.rs:1957`, turn the
      `ITERABLES` element at `crates/nvs-stdlib/src/registry.rs:1997` into `CoreTy::Var("T")`, and
      make `all`/`first` at `crates/nvs-stdlib/src/db.rs:594` and `crates/nvs-stdlib/src/db.rs:603`
      answer `array<T>`/`?T`. `CoreTy::Instance` at `crates/nvs-stdlib/src/registry.rs:498` interns
      a generic class at its own variables (`crates/nvs-types/src/core_lib.rs:453`), so `query`'s
      `Rows<Row>` needs a sibling variant carrying its arguments — and
      `crates/nvs-stdlib/src/registry.rs:909`'s `CoreMethod::written` must descend into it, or
      `queryAs`'s `T` is invisible there and the call is refused as non-generic. ADR 0067 § 18,
      spec § 18's *Results* table.
- [ ] **The `queryAs` row, on `Core\Db\Queryable`'s two classes.** `CONNECTION` is
      `crates/nvs-stdlib/src/db.rs:324` and `TRANSACTION`'s forward of `query` is
      `crates/nvs-stdlib/src/db.rs:445`. The class written at the call site reaches the helper
      through `crates/nvs-stdlib/src/registry.rs:1924`'s `WRITTEN_CLASS_MEMBERS`, whose one row
      today is `Core\Json::decodeAs` — its helper takes the `ClassDesc` in slot 0 and a list flag
      in slot 1 ahead of its declared parameters (`crates/nvs-stdlib/src/json.rs:893`). `queryAs`
      is that roster's first *instance* member, so check where `nvs-ir` writes those two slots
      relative to the receiver before writing the arity.
- [ ] **The hydration body, then `examples/db.nvs` end to end.**
      `crates/nvs-stdlib/src/db.rs:2287` is `query`'s helper, which this copies down to the row
      array; each row then fills one constructor from `ClassDesc::db_codec()`, on
      `crates/nvs-stdlib/src/json.rs:939`'s `decode_as` shape — a codec-less class and an
      `Opaque` field are its two refusals — except that the per-field read is § 6's typed reader
      off a `Core\Db\Row` rather than a document walk. Then the fixture, which is the first
      execution of the `foreach` compiled two sessions ago.

## Backlog

- `Connection`'s `close()`, `driver()`, `serverVersion()` and `isOpen()` — spec § 18's table.
- `Rows::columns()` on a `ColumnType` enum — spec § 18's *Results* table, the sixth of six.
- `open`'s shape-parameter `CoreTy` — ADR 0067 § 2, the settings union.
- `stream`/`streamAs` — ADR 0067 § 4's cursor form, `Iterable<Row>`/`Iterable<T>`.
- The per-core pool and its reset — ADR 0067 § 13, Stages 3 to 7.
- `Core\Queue` and ADR 0084's durable jobs — the acceptance line's own leg, Stage 8.
