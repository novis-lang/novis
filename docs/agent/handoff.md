# Handoff

## State

**`Core\Db\Rows` is generic at `T`, and `Core\Db\Connection::query` answers `Rows<Row>`.** Spec § 18's
`Rows` and `Rows<T>` are one class now, not two: `all`/`first` answer `array<T>`/`?T`, the
`ITERABLES` row is `CoreTy::Var("T")`, and the receiver's argument is substituted in by the machinery
`Core\ObjectSet` already went through. The `ROWS` class doc owns the reasoning.

**A registry row can now write an instance at concrete arguments** — `CoreTy::InstanceAt(name, args)`,
beside `CoreTy::Instance` rather than replacing it, because a generic class reached through the bare
variant interns at its *own* variables and that is still right for a member whose receiver supplies
them. `CoreMethod::written` descends into it, so `queryAs<T>`'s `Rows<T>` will make the call generic;
`every_instance_type_names_a_registered_class` checks each row's argument count against
`GENERIC_CLASSES`.

**Bare `Core\Db\Rows` in a program is now `E0442`** — three conformance cases were edited to write
`Core\Db\Rows<Core\Db\Row>`, and the playbook bullet owns the general shape of that cost.

**`examples/db.nvs` still stops on `queryAs<T>` alone**, and both halves it needs are now on disk: ADR
0071's row codec at `ClassDesc::db_codec()`, which nothing reads yet, and the return type this session
made spellable. The next group spends both.

**The driver's acceptance line names `examples/queue.nvs`, and that is Stage 8's unlanded queue, not a
regression** — `Core\Queue` has never existed and everything behind it stays unchecked until ADR 0084
lands. **The CA is still not in git**; `nvs_host::tls`'s module doc owns why.

## Next group

**`queryAs<T>` in two slices — the file set is `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-stdlib/src/registry.rs` and `crates/nvs-stdlib/src/json.rs`, with `examples/db.nvs` as the
proof.**

- [ ] **The `queryAs` row, on `Core\Db\Queryable`'s two classes.** `CONNECTION` is
      `crates/nvs-stdlib/src/db.rs:324` and `TRANSACTION` is `crates/nvs-stdlib/src/db.rs:444`; both
      forward `query` to one helper and this member does the same. The return is
      `CoreTy::InstanceAt(ROWS_NAME, &[CoreTy::Written("T")])` — `crates/nvs-stdlib/src/db.rs:347` is
      the `Rows<Row>` sibling to copy. The class written at the call site reaches the helper through
      `crates/nvs-stdlib/src/registry.rs:1944`'s `WRITTEN_CLASS_MEMBERS`, whose one row today is
      `Core\Json::decodeAs` — its helper takes the `ClassDesc` in slot 0 and a list flag in slot 1
      ahead of its declared parameters (`crates/nvs-stdlib/src/json.rs:893`). `queryAs` is that
      roster's first *instance* member, so check where `nvs-ir` writes those two slots relative to the
      receiver before writing the arity. ADR 0067 § 4, spec § 18's *Queryable* table.
- [ ] **The hydration body, then `examples/db.nvs` end to end.**
      `crates/nvs-stdlib/src/db.rs:2311` is `query`'s helper, which this copies down to the row array;
      each row then fills one constructor from `ClassDesc::db_codec()`, on
      `crates/nvs-stdlib/src/json.rs:939`'s `decode_as` shape — a codec-less class and an `Opaque`
      field are its two refusals — except that the per-field read is § 6's typed reader off a
      `Core\Db\Row` rather than a document walk. Then the fixture, which is the first execution of the
      `foreach` compiled three sessions ago. ADR 0071 § 3, ADR 0067 § 6.

## Backlog

- `Connection`'s `close()`, `driver()`, `serverVersion()` and `isOpen()` — spec § 18's table.
- `Rows::columns()` on a `ColumnType` enum — spec § 18's *Results* table, the sixth of six.
- `open`'s shape-parameter `CoreTy` — ADR 0067 § 2, the settings union.
- `stream`/`streamAs` — ADR 0067 § 4's cursor form, `Iterable<Row>`/`Iterable<T>`.
- The per-core pool and its reset — ADR 0067 § 13, Stages 3 to 7.
- `Core\Queue` and ADR 0084's durable jobs — the acceptance line's own leg, Stage 8.
