# Handoff

## State

**M4's Stage 7 is under way: ADR 0079 § 1 is closed but for its parameter
bullet, and § 4's assertion surface is registered and runs.** `#[Test]` is
recognized, its payload checked, its table built, and four of § 1's five compile
errors made. `Core\Test` now has a `CoreClass` row (`nvs_stdlib::test`), so
`Test::assertEquals` resolves in a body — the attribute and the assertion class
are the same `QName` and one `use Core\Test;` places both.

- **Nothing reads the `#[Test]` table yet.** `ExprTypeTable::tests(label)`
  (`crates/nvs-types/src/expr_table.rs:779`) and `test_classes()` are the
  accessors and the runner is what will call them, so the rows are asserted in
  `crates/nvs-types/tests/testing.rs` rather than by a `.nvst`. Stage 8's named
  `a-test-attribute-builds-a-table-the-runner-reports.nvst` still waits on that
  runner and is the one named case `python tools/loop.py --list` reports missing.
- **A failed assertion is a throw and nothing more.** ADR 0079 § 5's ledger does
  not exist, so `try { … } catch (Throwable $t) {}` around an assertion still
  hides it — the silently-passing test that section exists to abolish. That, the
  `Core\Test\Failure` class and `Core\Test::expectFailure(callable)` are the next
  slice; `nvs_stdlib::test`'s known gap 2 is its one home.
- § 4's roster is the three equality members only. The
  `assertTrue`/`assertNull`/`assertCount`/`assertThrows` that section's example
  also writes are the same shape and are owed.
- § 1's remaining compile error — a `#[Test]` parameter no `#[Fixture]` supplies
  and no data row fills — is not decidable in `nvs_types::testing` at all and
  belongs with §§ 8-9.
- The conformance corpus is at **728**.

## Next group

**ADR 0079 § 5's ledger, then the runner.** New file set, and it is a crate
further down than § 4's was: `crates/nvs-runtime/src/ctx.rs:210` (`Ctx`, which is
where per-test state a program cannot reach has to live),
`crates/nvs-runtime/src/throwable.rs:92` (`ThrownClass`, spec § 10's tree),
`crates/nvs-types/src/error_lib.rs` (what seeds a catchable class name), and
`crates/nvs-stdlib/src/test.rs` (`failed`, the one site every failure goes
through) with `docs/adr/0079-testing-is-a-language-feature.md` §§ 5, 20, 22.

- [ ] **`Core\Test\Failure` is a named catchable class** (ADR 0079 § 5) — the
      example catches it by name, so it needs a `ThrownClass` variant
      (`throwable.rs:92`) and a seeding in `nvs_types::error_lib` beside spec
      § 10's tree, and `nvs_stdlib::test::failed` (`test.rs`, the one throw site)
      raises `Fault::thrown_as` with it instead of the bare `Fault::thrown` it
      raises today.
- [ ] **The ledger the catch cannot erase** (ADR 0079 § 5) — every assertion
      records its outcome on `Ctx` (`ctx.rs:210`, beside the pending-exception
      state at `:779`/`:906`, which is deliberately *not* what the runner reads),
      plus `Core\Test::expectFailure(callable)` as the one greppable spelling
      that consumes a failure on purpose and removes its entry. `call_closure` is
      how a `Core` member runs a `callable`.
- [ ] **The runner** (ADR 0079 §§ 20, 22) — what reads
      `ExprTypeTable::tests`/`test_classes` (`expr_table.rs:779`), reports in
      declaration order, and reads the *ledger* rather than the exception state
      at the end of each test. This is what unblocks Stage 8's named
      `a-test-attribute-builds-a-table-the-runner-reports.nvst`.

## Backlog

- § 4's other assertions (`assertTrue`, `assertNull`, `assertCount`,
  `assertThrows`) — `docs/adr/0079-testing-is-a-language-feature.md` § 4.
- `assertEquals` on a non-`Comparable` object should be refused where it is
  written — ADR 0079 § 4, and `nvs_stdlib::test`'s known gap 1.
- A `require` whose path is not a string literal runs nothing, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- ADR 0033's container axis: a `secret` inside an `array<T>` or an ADR 0036 shape
  field carries no bit — `nvs_stdlib::debug`'s known gap 1.
- `Core\Uuid` has no `bytes` round trip — `nvs_stdlib::uuid`'s known gap 1.
