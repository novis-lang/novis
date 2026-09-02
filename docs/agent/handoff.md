# Handoff

## State

**`Core\Db\DbError` is a class in spec § 10's tree** — the row at `crates/nvs-hir/src/errors.rs:82`
under `RuntimeError` beside `Core\Db\RolledBack`, with `nvs_runtime::ThrownClass::DbError`
(`crates/nvs-runtime/src/throwable.rs:146`) as the runtime half. It declares **no property of its
own**: § 18's `kind`, `sqlState`, `driverCode`, `constraint` and `sql` each owe a seeded type in
`nvs_types::error_lib` first, which `nvs_hir::errors::OWN_PROPERTIES`'s doc states.

**Every refusal the server itself made now throws it** — `statement_failure`'s `Other` arm at
`crates/nvs-stdlib/src/db.rs:2514`, which is the one place `query`, `queryAs`, `execute`,
`executeMany` and `transaction`'s `BEGIN`/`COMMIT` all render a refusal. Nothing that caught
`RuntimeError` stops matching, the new class being its child; the wire's own failures stay `IOError`
and the call's own mistakes stay `LogicError`.

**Gap 4 (`crates/nvs-stdlib/src/db.rs:85`) has narrowed to those five properties, and gap 9's second
half is untouched.** The retry loop still branches on the *commit's* refusal
(`crates/nvs-stdlib/src/db.rs:3441`, `nvs_db::ServerError::of`) because a fault out of the closure
carries no kind — `statement_failure` holds the kind at the instant it throws and drops it. The next
group is exactly that hole.

**No case pins the new class on a live refusal**: that needs the `[db.main]` fixture and a
multi-file case, so only the tree-shape case
`tests/conformance/error/a-db-error-is-in-the-tree-beside-a-rolled-back.nvst` is on disk.

The driver's acceptance line still names `examples/queue.nvs` — Stage 8's unlanded `Core\Queue`
(ADR 0084), not a regression. Stage 5's `args = ["test", "-p", "nvs-db"]`
(`docs/agent/loop-goal.toml:2830`) still cannot see the two `nvs-stdlib` tests, and is still the
user's call. The CA is still not in git; `nvs_host::tls`'s module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 8, which specified every slice of this group,
and not § 7, which the previous handoff already asked for.** Add `0067 § 7` and `0067 § 8` to
`[context] adrs` in `docs/agent/loop-goal.toml`; both were sliced by hand again.

## Next group

**Give `Core\Db\DbError` § 18's `kind`, which is the one thing the retry loop is waiting for. The
file set is `crates/nvs-hir/src/errors.rs`, `crates/nvs-types/src/error_lib.rs`,
`crates/nvs-ir/src/lower/exception.rs` and `crates/nvs-stdlib/src/db.rs`.**

- [ ] **`kind` becomes an own property** — a row in `crates/nvs-hir/src/errors.rs:139`'s
      `OWN_PROPERTIES` with a `KIND_SLOT` beside `crates/nvs-hir/src/errors.rs:167`'s `REASON_SLOT`,
      and the type arm in `crates/nvs-types/src/error_lib.rs:102`'s `own_properties` that seeding
      `panic!`s without. **Decide there what an `ErrorKind` value is**: `Core\Db\Isolation`
      (`crates/nvs-stdlib/src/db.rs:684`) is the registered-enum precedent, and a `string` naming
      the kind is the fallback spec § 18 does not spell. ADR 0067 § 8.
- [ ] **The synthesized constructor writes it** — `crates/nvs-ir/src/lower/exception.rs:687` is the
      roster, and `crates/nvs-ir/src/lower/exception.rs:699` is how `RolledBack::$reason` gets the
      message. The playbook's *Running things* bullet on a `TREE` row with own properties lists the
      rest: the *functions* list in `nvs-ir`'s
      `a_file_with_no_class_still_carries_every_compiler_declared_class`, and five insta snapshots
      that `INSTA_UPDATE=always cargo test -p nvs-ir --lib` rewrites.
- [ ] **`statement_failure` fills the slot** — `crates/nvs-stdlib/src/db.rs:2508` already has the
      kind in hand (`nvs_db::ServerError::of(refused)`) and throws a `Fault` that carries a message
      and nothing else, so this slice is about what a native throw may write. ADR 0067 § 8.
- [ ] **The retry loop reads the closure's conflict** — `crates/nvs-stdlib/src/db.rs:3394`'s
      `Err(fault)` arm rolls back and re-raises; with a kind reachable it becomes the same
      four-condition test `crates/nvs-stdlib/src/db.rs:3443` already runs, and gap 9
      (`crates/nvs-stdlib/src/db.rs:132`) loses its second half. ADR 0067 § 7.

## Backlog

- A live-server case pinning `catch (Core\Db\DbError)` on a real refusal — `docs/agent/loop-goal.toml`'s Stage 5 owns the check.
- `DbError`'s other four § 18 properties and § 5's `issues` — gap 4, `crates/nvs-stdlib/src/db.rs:85`.
- `open` waits on a shape-parameter registry type — gap 1, same module doc.
- `stream`/`streamAs`, `close` and § 18's three `Connection` properties — gap 5, same module doc.
- Stage 5's `args` cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`, the user's call.
- Stage 8's `Core\Queue` (ADR 0084) is what the driver's acceptance line names.
