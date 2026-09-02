# Handoff

## State

**§ 8's `kind` and `RolledBack`'s `$reason` are now pinned in programs, not only in `-p` unit
tests.** `examples/db.nvs:146` writes row 1 of `people` a second time inside a `try`, catches
`Core\Db\DbError` and `match`es `$refused->kind` against `Core\Db\ErrorKind::UniqueViolation`; it
prints `kind=UniqueViolation` against `tests/db/compose.yaml`'s PostgreSQL, so the whole path — the
server's `23505`, `nvs_db`'s normalisation, `Fault::thrown_with_slot` at
`crates/nvs-stdlib/src/db.rs:2637`, `KIND_SLOT`, and the compiler's enum-typed property — is held by
one line of output. `examples/transaction.nvs:62` reads `$rolled->reason` off the `Db\RolledBack`
that `rollBack("…")` threw and prints it. Both lines are frozen in `docs/agent/loop-goal.toml` (the
`examples/db.nvs` check under stage 9, the `examples/transaction.nvs` one under stage 5) and in
`docs/agent/goals/5-database.toml` beside it — the second copy is not optional, because
`goal-switch.py` restores the live file from it.

**§ 8's four raw values are still a decision, not a fill-in.** `sqlState`, `driverCode`,
`constraint` and `sql` are four more slots on `Core\Db\DbError`, and `Fault::ThrownWithSlot`
(`crates/nvs-runtime/src/abi.rs:108`) carries exactly one — deliberately. Widening it, seeding the
rest at `Thrown::new_as`, or leaving three of them unlanded are three different answers with
different costs, and the next group's first slice is to pick one and write it down. Nothing about
`kind` moves either way: `crates/nvs-hir/src/errors.rs:186` already says `KIND_SLOT` stays 0
relative to the root as the four land after it.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression; its `[[check]]` block is
`docs/agent/loop-goal.toml:2927`. Stage 5's `args = ["test", "-p", "nvs-db"]`
(`docs/agent/loop-goal.toml:2830`) still cannot see the two `nvs-stdlib` tests, and is still the
user's call. The CA is still not in git; `nvs_host::tls`'s module doc owns why.

**`orient.py` printed ADR 0067 §§ 1, 9 and 13 — not § 7 or § 8, which specify this group and the
last two.** Five handoffs have now asked: add `0067 § 7` and `0067 § 8` to `[context] adrs` in
`docs/agent/loop-goal.toml`.

## Next group

**Decide what a wider throw costs, then land the two raw values that pay for themselves. The file
set is `crates/nvs-runtime/src/abi.rs`, `crates/nvs-hir/src/errors.rs`,
`crates/nvs-types/src/error_lib.rs` and `crates/nvs-stdlib/src/db.rs`.**

- [ ] **Decide what § 8's four raw values cost, and record the decision** — `Fault::ThrownWithSlot`
      at `crates/nvs-runtime/src/abi.rs:108` carries one `(slot, value)` pair and
      `Fault::thrown_with_slot` at `crates/nvs-runtime/src/abi.rs:157` is its only constructor. The
      choice is a second variant carrying a slice, a wider tuple, or seeding the extra slots inside
      `Thrown::new_as` (`crates/nvs-runtime/src/throwable.rs:335`) the way `RolledBack`'s `$reason`
      already is. Weigh it as ADR 0002's cost on the ordinary throw path, not `Core\Db`'s alone, and
      write the answer into ADR 0067 § 8's body. ADR 0067 § 8.
- [ ] **`sqlState` and `driverCode` become slots 1 and 2** — append to `KIND` at
      `crates/nvs-hir/src/errors.rs:155`, give each a seeded type beside `"kind"` at
      `crates/nvs-types/src/error_lib.rs:124` (both are `string`; a row without an arm there
      `panic!`s at seed time), and fill them at the raiser,
      `crates/nvs-stdlib/src/db.rs:2637`. ADR 0067 § 8.
- [ ] **A leg pinning `sqlState` beside `kind`** — extend the `catch` at `examples/db.nvs:155` to
      echo the raw code as well, and add the line to both `want` lists. Only once the slice above
      lands. ADR 0067 § 8.

## Backlog

- The `transaction` retry has no backoff — `crates/nvs-stdlib/src/db.rs` module gap 9 owns it.
- The pool, ADR 0067 § 13 — stages 5 to 7 of `docs/agent/loop-goal.toml`.
- `open` still waits on a shape-parameter type — `docs/implementation-plan.md`, Open now.
- `constraint` and `sql`, § 8's other two raw values — ADR 0067 § 8.
- The four drivers that are not PostgreSQL — stage 6, `tools/db-matrix.py --all`.
- The CA is not in git; `crates/nvs-host/src/tls.rs`'s module doc owns why.
