# Handoff

## State

**`examples/db.nvs` compiles as far as `queryAs<T>`, which is the one thing it still needs.**
`Core\Db\Rows` is `Iterable<Row>` — a row in `nvs_stdlib::registry::ITERABLES` plus
`nvs_core_db_rows_iterate` in the dispatch roster, which is the same two-roster shape `Core\IO\Lines`
already had — and `Write`'s three readers are settled: `docs/spec/01-core-library.md` § 18 wrote them
as readonly properties, a `Core` instance has no property a program can reach
(`nvs_stdlib::registry::CoreTy::Instance`), so the spec's rows now write the readers the registry
carries and its preamble states the rule once for `Decision` and `ExitReport` too.

**The runtime half of that `foreach` has been compiled but never run.** Nothing executes it until
`examples/db.nvs` runs whole, because a `Rows` needs a live server and the fixture is the only granted
path; `every_iterable_class_answers_the_iteration_protocol` pins the roster pairing and nothing else.

**The driver's acceptance line names `examples/queue.nvs`, and that is Stage 8's unlanded queue, not a
regression** — `Core\Queue` has never existed. It sits one program leg ahead of `examples/db.nvs` in
`docs/agent/loop-goal.toml`, so it will keep being the reported failure, and everything behind it stays
unchecked, until ADR 0084 lands. The playbook bullet owns the reading.

**The CA is still not in git** — it belongs to the `certs` volume, and `nvs_host::tls`'s module doc
owns why a checkout that has never brought the fixtures up cannot boot from the repo root.

## Next group

**What `examples/db.nvs` still needs, then running it — the file set is `crates/nvs-stdlib/src/db.rs`
and `examples/db.nvs`, with one new checker path beside them.**

- [ ] **`queryAs<T>`** — `crates/nvs-stdlib/src/db.rs:324` is the `CONNECTION` class and
      `crates/nvs-stdlib/src/db.rs:2287` `query`'s helper, which it hydrates over. **It does not want
      `open`'s shape-parameter `CoreTy`**: the type argument is written at the *call*, not declared in
      a signature, and the landed precedent for that is `Core\Attributes::get<T>`, which
      `crates/nvs-types/src/retrieval.rs:286` answers as a checker special case rather than as a
      registry feature. So this is a `CoreTy` for the return (`Rows<T>` over the written argument) plus
      a hydration body calling the class's `Db\Codec::fromRow`; ADR 0067 §§ 4 and 6, spec § 18.
- [ ] **Run `examples/db.nvs` end to end against the compose fixture**, which is the first execution of
      `crates/nvs-stdlib/src/db.rs:2781`'s cursor — `examples/db.nvs:69` is the `foreach`. Want
      `rows=3`, `ada`, `grace`, `alan`, `affected=1`, `typed row ok`. A refcount slip here is a
      valgrind item, so pair it with the WSL leg per `docs/agent/commands.md`.
- [ ] **`Connection`'s `close()`, `driver()`, `serverVersion()` and `isOpen()`** —
      `crates/nvs-stdlib/src/db.rs:324` again, and `crates/nvs-stdlib/src/db.rs:378` is its slot list.
      Spec § 18's Connection row, now written as readers. Small, same file, and it closes four of the
      names this module's known gap 5 lists.

## Backlog

- `Rows::columns()` needs a `Core\Db\Column`, a `Core\ColumnType` and an OID classification `nvs-db`
  does not expose — `crates/nvs-stdlib/src/db.rs`'s known gap 5.
- `open` still waits on a shape-parameter `CoreTy` — same known gap 5.
- `stream`/`streamAs` hold the connection until drained — ADR 0067 § 4.
- § 9's five structured rows do not read back — known gap 6.
- `Db\DbError` is not in spec § 10's tree — known gap 4.
- Stage 8's `Core\Queue` is a whole stage of its own — ADR 0084.
