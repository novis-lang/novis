# Handoff

## State

**M4's Stage 7 is started: ADR 0079 § 1's `#[Test]` is recognized, its payload is
checked and the table is built.** `Core\Test` is on
`nvs_types::derive::ATTRIBUTES:77`, ADR 0071 § 1's table carries the row, and
`nvs_types::testing` owns both the option roster and the collection pass, which
`nvs_types::check:227` runs beside `check_class_derive`.

- **Nothing reads the table yet.** `ExprTypeTable::tests(label)` and
  `test_classes()` are the accessors and the runner is what will call them, so
  the rows are asserted in `crates/nvs-types/tests/testing.rs` rather than by a
  `.nvst`. Stage 8's named
  `a-test-attribute-builds-a-table-the-runner-reports.nvst` still waits on that
  runner and is the one named case `python tools/loop.py --list` reports missing.
- `Core\Test` has **no row in `nvs_stdlib::registry`**, so `Test::assertEquals`
  in a body does not resolve — the attribute name and the assertion class are
  the same `QName` on purpose, but only the attribute half exists.
- The conformance corpus is at **726**.

## Next group

**ADR 0079 § 1's five compile errors, then the assertion roster.** The first two
share one file set with what just landed:
`crates/nvs-types/src/testing.rs`, `check.rs:227`, plus
`docs/adr/0079-testing-is-a-language-feature.md`.

- [ ] **§ 1's five compile errors** (ADR 0079 § 1's bullet list) — two `#[Test]`
      methods with the same name in one class, and a `#[Test]` that is `static`,
      that returns anything but `void`, or that is not `public`. All five are
      decidable in `testing::check_class_tests`, which already walks exactly
      these members and holds each one's `MethodMember` (`crates/nvs-types/src/testing.rs:129`);
      the parameter bullet waits on §§ 8-9's `#[Fixture]` and belongs with them.
      `E0733` is the next free `E07xx` and one code covers the "shape a test
      method must have" family, the duplicate name taking `E_DUPLICATE_DECLARATION`
      as the option-twice refusal already does.
- [ ] **`Core\Test`'s assertion roster** (ADR 0079 § 2) — the class needs rows in
      `nvs_stdlib::registry` before any `.nvst` body can call one, and Stage 7's
      second guard names `cargo test -p nvs-stdlib (the assertion roster)`
      (`docs/agent/loop-goal.toml:906`).
- [ ] **The runner** (ADR 0079 §§ 3-4) — what reads `ExprTypeTable::tests` and
      what finally lets Stage 8's named case be written.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently,
  in both forms — `nvs_hir::requires`' own known gap.
- `nvs_stdlib::debug`'s gap 1: an ADR 0036 shape field and an `array<T>` element
  carry no `secret` bit, ADR 0033's unmodelled container axis.
- `nvs_types::derive`'s gap 1: a promoted constructor parameter is not a
  `#[Json\Derive]` field, though `crate::layout` now gives one a slot.
- ADR 0071 § 1's table names eleven attributes and `ATTRIBUTES` carries three;
  each of the other eight owes the pass behind it (`derive.rs`'s gap 3).
