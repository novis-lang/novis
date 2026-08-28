# Handoff

## State

**M4's Stage 7 is under way: ADR 0079 §§ 1, 4 and 5 are closed but for § 1's
parameter bullet, and the runner is what is left.** `#[Test]` is recognized and
its table built, `Core\Test`'s three equality members run, and a failed
assertion now throws the named `Core\Test\Failure` *and* records a ledger entry
`nvs_runtime::Ctx` holds and no `Core` member reads back.

- **`Core\Test\Failure` is a row of `nvs_hir::errors::TREE`**, the tree's one
  namespaced entry, so a `catch` by name, `new`, `instanceof Throwable` and the
  inherited constructor all fell out of the table rather than out of new cases.
  `QName::is_core` is what makes it trusted-to-exist;
  `is_reserved_global_class` stays single-segment on purpose.
- **Nothing reads the ledger yet**, which is `nvs_stdlib::test`'s known gap 2:
  `Ctx::take_assertions` is the accessor and the runner is what will call it, so
  § 20's zero-assertion rule is owed rather than broken. Stage 8's named
  `a-test-attribute-builds-a-table-the-runner-reports.nvst` still waits on that
  runner and is the one named case `python tools/loop.py --list` reports missing.
- § 4's roster is still the three equality members only. The
  `assertTrue`/`assertNull`/`assertCount`/`assertThrows` that section's example
  also writes are the same shape and are owed —
  `nvs_stdlib::test`'s known gap 3.
- § 1's remaining compile error — a `#[Test]` parameter no `#[Fixture]` supplies
  and no data row fills — is not decidable in `nvs_types::testing` at all and
  belongs with §§ 8-9.
- The conformance corpus is at **730**.

## Next group

**The runner, §§ 20 and 22.** It is the reader every landed half is waiting
for, and its file set is the two this session left in hand plus the CLI:
`crates/nvs-types/src/expr_table.rs:779` (`ExprTypeTable::tests(label)` and
`test_classes()`, the rows nothing calls),
`crates/nvs-runtime/src/ctx.rs` (`Ctx::take_assertions`, the ledger's one
reader-to-be, beside `record_assertion`), `crates/nvs-stdlib/src/test.rs` and
`crates/nvs-cli/src/` with ADR 0079 §§ 20, 22.

- [ ] **The runner constructs each `#[Test]` class and calls its methods**
      (ADR 0079 § 20) — declaration order, no arguments, no result, one
      instance per test method; `ExprTypeTable::tests` is the roster and
      `nvs-cli` is where a subcommand can reach both it and a `Ctx`.
- [ ] **A test's verdict is read off the ledger, not the exception state**
      (ADR 0079 § 5) — `Ctx::take_assertions` between tests, so a `catch`
      around an assertion still reports FAILED, and a test that produced no
      entry at all fails under § 20's zero-assertion rule.
- [ ] **`a-test-attribute-builds-a-table-the-runner-reports.nvst`** (ADR 0079
      §§ 1, 20) — Stage 8's one named case still missing, and the first thing
      that can observe a `#[Test]` row or its order at all.

## Backlog

- `assertTrue`/`assertNull`/`assertCount`/`assertThrows` — `nvs_stdlib::test`
  known gap 3, ADR 0079 § 4.
- A non-`Comparable` object under `assertEquals` is a throw where the ADR
  refuses the program — `nvs_stdlib::test` known gap 1.
- A `#[Test]` parameter no `#[Fixture]` supplies — ADR 0079 §§ 8-9.
- A `require` whose path is not a string literal runs nothing, silently —
  `nvs_hir::requires` known gap.
- ADR 0092 § 6's `Throwable` record producer — `nvs_stdlib::debug`'s own note.
- A shape literal's field carries no `secret` bit — `nvs_stdlib::debug` gap 1.
