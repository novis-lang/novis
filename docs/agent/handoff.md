# Handoff

## State

**A pending throw's extra slot is readable without consuming the throw, and ADR 0067 § 7's retry
now re-runs the closure's own conflict.** `Ctx::pending_slot(class, slot)`
(`crates/nvs-runtime/src/ctx.rs:3499`) is `take_thrown`'s borrowing half: it answers `None` unless
the pending failure conforms to the class named, which is the whole of what keeps `KIND_SLOT` from
reading a `ParseError`'s `issues` back as an `ErrorKind` — the two are the same slot number. It
reads through `Thrown::field` (`crates/nvs-runtime/src/throwable.rs:459`), guarded by the same
`count > slot` bound `new_as` writes under, and takes no reference: the value is good only while
the failure is still pending, which is all a decide-then-re-raise caller needs.

**`transaction` retries both conflicts** (`crates/nvs-stdlib/src/db.rs:3561`). The commit's refusal
is an `io::Error` carrying its `nvs_db::ServerError`; the closure's is a pending `Db\DbError`, so
its kind is read off the object and turned back into a `nvs_db::DbErrorKind` by `error_kind_of`
(`crates/nvs-stdlib/src/db.rs:2662`) — the inverse of `error_kind_value`, computed *through* it
rather than as a second name table. Both then ask `is_retryable`, and the four conditions are
unchanged. A retry clears the pending first, because the decision is that the throw did not happen.
Module gap 9 now holds only the backoff.

**A natively thrown `Core\Db\RolledBack` carries its `$reason`.** `Thrown::new_as` seeds it from the
message (`crates/nvs-runtime/src/throwable.rs:335`), which is the same text `nvs_ir`'s
`ExtraInit::Message` gives a hand-built one, so the two ways of building that class no longer
disagree. `nvs_runtime::REASON_SLOT` is the compiler's `nvs_hir::errors::REASON_SLOT`, held to it by
`the_runtime_and_the_compiler_agree_on_every_throwable_slot`.

**Nothing pins any of this in a program.** All three are held by `-p` unit tests only. The `kind` on
a live refusal and the `$reason` on a real `rollBack` both need a database, so their home is a
program leg beside `examples/db.nvs` rather than `tests/conformance/`, which CI runs on three
runners with no PostgreSQL. That is the next group's first two slices.

**§ 8's four raw values still owe more than a seeded type.** `sqlState`, `driverCode`, `constraint`
and `sql` are four more slots, and `Fault::ThrownWithSlot` (`crates/nvs-runtime/src/abi.rs:96`)
carries exactly one — deliberately, one session ago. Widening it is a decision, not a fill-in.

The driver's acceptance line still names `examples/queue.nvs` — Stage 8's unlanded `Core\Queue`
(ADR 0084), not a regression; its `[[check]]` block is `docs/agent/loop-goal.toml:2927`. Stage 5's
`args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) still cannot see the two
`nvs-stdlib` tests, and is still the user's call. The CA is still not in git; `nvs_host::tls`'s
module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 7 or § 8, which specify every slice of this
group and the last one's.** Four handoffs have now asked: add `0067 § 7` and `0067 § 8` to
`[context] adrs` in `docs/agent/loop-goal.toml`.

## Next group

**Prove the two properties in a program, then decide what a wider throw costs. The file set is
`examples/db.nvs`, `examples/transaction.nvs` and `crates/nvs-stdlib/src/db.rs`.**

- [ ] **A leg pinning `kind` on a real refusal** — append to `examples/db.nvs:143`: insert a row
      twice inside a `try`, `catch (Core\Db\DbError $e)` and `match ($e->kind)` against
      `Core\Db\ErrorKind::UniqueViolation`. It runs against `tests/db/compose.yaml`'s PostgreSQL,
      which is up. ADR 0067 § 8.
- [ ] **A leg pinning `$reason` on a real `rollBack`** — `examples/transaction.nvs:62` already
      catches `Core\Db\RolledBack`; echo `$rolled->reason` beside the message it prints, which is
      the seeded value and was `null` before this session. ADR 0067 § 7.
- [ ] **Decide what § 8's four raw values cost** — `crates/nvs-runtime/src/abi.rs:96`'s
      `Fault::ThrownWithSlot` carries one slot and `sqlState`, `driverCode`, `constraint` and `sql`
      are four. Record the shape (a small slice of pairs, or a `DbError`-specific carrier) in the
      module doc at `crates/nvs-stdlib/src/db.rs:85` before writing any of it. ADR 0067 § 8.

## Backlog

- Stage 8's `Core\Queue` is what the failing acceptance check wants — ADR 0084.
- `stream`/`streamAs`, `close` and `Connection`'s three readonly properties — db module gap 5.
- `open` waits on a shape-parameter type — db module gap 1.
- § 7's backoff needs a helper that can suspend — db module gap 9, `nvs-runtime` gap 3.
- `queryAs<T>`'s three run-time refusals want a diagnostic band — db module gap 8.
- Only PostgreSQL opens; the other four drivers are Stage 6 — db module gap 2.
