# Handoff

## State

**`Core\Db\RolledBack` is in spec § 10's tree.** The row is `crates/nvs-hir/src/errors.rs:80`,
under `RuntimeError`. The whole roster a § 10 class owes is landed — the playbook's *Adding a row
to `nvs_hir::errors::TREE`* bullet enumerates six sites, and the bullet added beside it names the
three more that a new *property* costs, plus the insta snapshots.

**A § 10 class may add a property, and the mechanism is `OWN_PROPERTIES`** — that was the question
the item said to settle first. `ParseError` was already the precedent and `errors.rs`' own doc had
anticipated `Core\Db\DbError` joining it. `REASON_SLOT` is `PROPERTIES.len()` for the reason
`ISSUES_SLOT` is, and the two are siblings: `rolled_back_s_own_slot_starts_after_the_root_s`
asserts it separately so a slot added to either cannot silently move the other.

**`reason` is initialized to the message, not to the empty string** — the one design call in the
slice, recorded on `ExtraInit` in `crates/nvs-ir/src/lower/exception.rs`. Spec § 18 gives the class
nothing else to carry, so `new Core\Db\RolledBack("cart is empty")` and § 7's
`rollBack("cart is empty")` agree, and a helper raising `ThrownClass::DbRolledBack` writes no
second slot. `ParseError::$issues` still starts empty, because a hand-raised one has no field list
to report — which is why the extras are an enum rather than one shared initializer.

**The acceptance check is unchanged**: `examples/transaction.nvs` still stops at `->transaction`
(`E0405`). Closing it is the group below, which this session did not start: it touches none of the
files this one loaded, and `crates/nvs-stdlib/src/db.rs` is a 2,000-line read on its own.

## Next group

**Closing the acceptance check — the file set is `crates/nvs-stdlib/src/db.rs` and
`crates/nvs-db/src/pg.rs`, which already carries § 7's `BEGIN`/`COMMIT`/`ROLLBACK` and its
`SAVEPOINT` depth.**

- [ ] **§ 7's `transaction`, and the `Core\Db\Transaction` it hands the closure** — ADR 0067 § 7.
      The row joins `Connection`'s roster at `crates/nvs-stdlib/src/db.rs:319`, the body sits
      beside `executeMany`'s at `crates/nvs-stdlib/src/db.rs:2051`, and `nvs-db`'s half is landed
      at `crates/nvs-db/src/pg.rs:2658`. `nvs_runtime::call_closure` invokes the block;
      `crates/nvs-stdlib/tests/allocation_policy.rs:1` names `closure_of`, the shape a
      `-p nvs-stdlib` test uses to hand a real callable over with no compiler in front of it.
- [ ] **`Transaction` delegates `Queryable` to its connection** (ADR 0043) — the three landed
      members are `query`, `execute` and `executeMany` at `crates/nvs-stdlib/src/db.rs:319`; each
      needs a `Transaction` row answering the same shape through the connection it holds.
- [ ] **`rollBack` raises `ThrownClass::DbRolledBack`** — the variant is
      `crates/nvs-runtime/src/throwable.rs:136` and the message *is* the reason, so nothing writes
      a slot. Known gap 4 at `crates/nvs-stdlib/src/db.rs:85` is the half that stops being true.

## Backlog

- `Core\Db\DbError` in § 10's tree with § 18's five readonly properties — `docs/spec/01-core-library.md:1231`; wants a `Core\Db\ErrorKind` enum first.
- `Rows::columns()`, blocked on a `Core\Db\Column` class and a `Core\ColumnType` enum — `db.rs`'s known gap 5.
- § 9's five structured rows do not read back — `db.rs`'s known gap 6.
- `open` waits on a shape-parameter type — `db.rs`'s known gap 1.
- Only PostgreSQL opens; the other four drivers have no connect path — `db.rs`'s known gap 2.
- ADR 0067 § 13's per-core pool, Stages 3 to 7 — `docs/plan/m8.md`.
