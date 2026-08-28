# Handoff

## State

**M4's Stage 7 is under way: ADR 0079 § 1 is closed but for its parameter
bullet.** `#[Test]` is recognized, its payload is checked against the option
roster, the table is built, and four of § 1's five compile errors are made —
three shapes under `E0733` (`nvs_types::testing::check_method_shape`) and the
duplicate name under `E_DUPLICATE_DECLARATION`.

- **Nothing reads the table yet.** `ExprTypeTable::tests(label)` and
  `test_classes()` are the accessors and the runner is what will call them, so
  the rows are asserted in `crates/nvs-types/tests/testing.rs` rather than by a
  `.nvst`. Stage 8's named
  `a-test-attribute-builds-a-table-the-runner-reports.nvst` still waits on that
  runner and is the one named case `python tools/loop.py --list` reports missing.
- `Core\Test` has **no row in `nvs_stdlib::registry`**, so `Test::assertEquals`
  in a body does not resolve — the attribute name and the assertion class are
  the same `QName` on purpose, but only the attribute half exists.
- § 1's remaining compile error — a `#[Test]` parameter no `#[Fixture]` supplies
  and no data row fills — is not decidable in `nvs_types::testing` at all and
  belongs with §§ 8-9.
- The conformance corpus is at **727**.

## Next group

**ADR 0079 § 4's assertion roster, then the runner.** New file set, shared
between the two: `crates/nvs-stdlib/src/registry.rs:709` (`CLASSES`) and
`:790` (`CONSTRUCTORS`), a new `crates/nvs-stdlib/src/test.rs` beside
`uuid.rs`, and `docs/adr/0079-testing-is-a-language-feature.md` §§ 4-5.

- [ ] **`Core\Test`'s three equality members** (ADR 0079 § 4) — the class needs
      a `CoreClass` row and its members registered, generic and subject-first,
      so `Test::assertEquals` resolves in a body at all. `GENERIC_CLASSES`
      (`registry.rs:952`) is where a class that takes type parameters is
      declared; `crates/nvs-stdlib/src/uuid.rs` is the shape a member module
      takes.
- [ ] **The failure ledger** (ADR 0079 § 5) — a failed assertion is a catchable
      `Throwable` plus a ledger entry the `catch` cannot erase, so the throw
      alone is not the record. Needs § 4 landed first.
- [ ] **The runner** (ADR 0079 §§ 20, 22) — what reads
      `ExprTypeTable::tests`/`test_classes` and reports in declaration order.
      This is what unblocks Stage 8's named
      `a-test-attribute-builds-a-table-the-runner-reports.nvst`.

## Backlog

- § 1's parameter bullet — waits on §§ 8-9's `#[Fixture]` and data rows
  (`docs/adr/0079-testing-is-a-language-feature.md` § 1).
- A `require` whose path is not a string literal runs nothing, silently, in both
  forms (`nvs_hir::requires`' own known gap).
- Stage 8's corpus floor is 750 (`docs/agent/loop-goal.md` § *Stage 8*).
- `docs/spec/02-php-migration.md`'s score, via `python tools/check-migration.py`.
