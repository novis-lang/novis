# Handoff

## State

**A native throw can carry one extra slot value, and `Core\Db\DbError::$kind` is the second caller.**
`Fault::ThrownWithIssues` is now `Fault::ThrownWithSlot(class, message, slot, value)`
(`crates/nvs-runtime/src/abi.rs:96`), generalised in place rather than given a sibling variant: a
variant per property costs an arm in `record_fault`, another in `nvs_stdlib::task`'s `call_child`
and one in every reader added later. `Ctx::raise_with_slot` (`crates/nvs-runtime/src/ctx.rs:3374`)
is its one destination, and `Thrown::new_as` takes an `Option<(usize, Value)>` and writes the slot
whenever the descriptor is wide enough (`crates/nvs-runtime/src/throwable.rs:324`).
`Fault::thrown_with_issues` survives unchanged as a thin wrapper at `ISSUES_SLOT`, so
`nvs_stdlib::json`'s five sites never name a slot index.

**`statement_failure` fills it** (`crates/nvs-stdlib/src/db.rs:2628`): a server refusal reads
`nvs_db::ServerError::of(refused)` and throws `Core\Db\DbError` with the driver's own kind in
`nvs_runtime::KIND_SLOT`. A refusal with no `ServerError` behind it reads as `Other`, which is § 8's
own definition of that case, so the property is written on every path and never `null`.
`error_kind_value` (`crates/nvs-stdlib/src/db.rs:2662`) is `column_type_value`'s shape exactly —
name correspondence here, ordinal looked up in `ERROR_KIND` — and
`every_db_error_kind_case_is_named` holds both directions. Module gap 4's first half is closed;
its other half (`sqlState`, `driverCode`, `constraint`, `sql`) still owes seeded types.

**What is still unwritten on a natively-thrown object is a *default*, not a value.**
`Thrown::new_as` seeds only `ParseError::$issues` when a thrower passes `None`; a `RolledBack` thrown
by `rollBack` (`crates/nvs-stdlib/src/db.rs:3595`, `:3653`) therefore reads `$reason` as `null`,
where the synthesized constructor's `ExtraInit::Message` gives a hand-built one the message. Two
paths, two answers, and only the ADR 0022 half is right. Slot 3 of the backlog.

**The retry loop still branches on the commit's refusal.** The closure's own `DbError` is now
readable in principle but arrives as a pending exception rather than an `io::Error`, so reading it
means a new accessor for a slot on `Ctx`'s pending `Thrown`. Module gap 9's last paragraph states
exactly that.

**No `.nvst` case pins `kind` on a live refusal** — and the home for one is an open question, not
just unwritten work: CI runs `tests/conformance/` on three runners with no PostgreSQL, so a case
that needs `[db.main]` may belong beside `examples/db.nvs` as a program leg instead.

The driver's acceptance line still names `examples/queue.nvs` — Stage 8's unlanded `Core\Queue`
(ADR 0084), not a regression; its `[[check]]` block is `docs/agent/loop-goal.toml:2927`. Stage 5's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) still cannot see the two
`nvs-stdlib` tests, and is still the user's call. The CA is still not in git; `nvs_host::tls`'s
module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 8, which specified every slice of this group.**
Three handoffs have now asked for it: add `0067 § 7` and `0067 § 8` to `[context] adrs` in
`docs/agent/loop-goal.toml`.

## Next group

**Read the kind back off a thrown object, so the retry loop and a test can both see it. The file set
is `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-runtime/src/throwable.rs` and
`crates/nvs-stdlib/src/db.rs`.**

- [ ] **A reader for one slot of the pending `Thrown`** — `crates/nvs-runtime/src/ctx.rs:3570`'s
      `take_thrown` is the only route to the object today and it *consumes* the pending, which the
      retry loop must not do while it is still deciding. Add the borrowing half beside
      `pending_class` (`crates/nvs-runtime/src/ctx.rs:3442`), guarded by the same
      `count > slot` check `Thrown::new_as` uses (`crates/nvs-runtime/src/throwable.rs:324`), and
      answering `None` for a class that is not the one asked about. ADR 0067 § 8.
- [ ] **The retry loop reads the closure's conflict** — `crates/nvs-stdlib/src/db.rs:3608` computes
      `conflicted` from the *commit's* `io::Error`. Widen it: where the closure itself failed with a
      pending `Core\Db\DbError`, map its `KIND_SLOT` ordinal back through `ERROR_KIND` to
      `nvs_db::DbErrorKind::is_retryable`. The pending must survive an attempt that is not retried,
      untouched. Then rewrite module gap 9's last paragraph. ADR 0067 § 7.
- [ ] **`new_as` seeds `RolledBack::$reason` from the message** — `crates/nvs-runtime/src/throwable.rs:324`'s
      `None` arm knows only `ParseError`. A natively thrown `RolledBack`
      (`crates/nvs-stdlib/src/db.rs:3595`, `crates/nvs-stdlib/src/db.rs:3653`) reads `$reason` as
      `null` where `nvs_ir::lower::exception`'s `ExtraInit::Message` gives a hand-built one the
      message; the two must agree. Spec § 18, ADR 0022.
- [ ] **A case pins `kind` on a real refusal** — decide the home first (see `## State`): a
      conformance case cannot reach PostgreSQL on CI. `examples/db.nvs` is the shape that already
      runs against `tests/db/compose.yaml`, and `docs/agent/loop-goal.toml:2830`'s Stage 5 is where
      a new program leg is registered. ADR 0067 § 8.

## Backlog

- `Db\DbError`'s four raw values (`sqlState`, `driverCode`, `constraint`, `sql`) each owe a seeded
  type in `nvs_types::error_lib::own_properties` — module gap 4's other half.
- § 7's backoff needs a helper that can suspend — `nvs-runtime`'s own known gap 3.
- `stream`/`streamAs`, `close` and § 18's three readonly `Connection` properties — module gap 5.
- Stage 5's `-p nvs-db` check cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- `examples/queue.nvs` needs Stage 8's `Core\Queue` — ADR 0084, `docs/agent/loop-goal.toml:2927`.
