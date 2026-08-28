# Handoff

## State

**M4's Stage 7: § 20's retry is closed, and what Stage 7 still owes is
§§ 8-9's fixtures.** `python tools/loop.py --list` reports no named `.nvst`
case owed by any stage.

- **A retried test is flaky, in all three formats at once.**
  `Outcome::Flaky` is a fifth verdict carrying its attempt count and the last
  failed attempt's failures; `crates/nvs-cli/src/runner.rs`'s
  `run_with_retries` is the home of why only a failure is retried, why the
  *last* attempt is what is reported, and why a flaky test does not fail the
  run. ADR 0079 § 20 carries the rule.
- **`retries:` with no `because:` is `E0734`**, checked once the payload has
  been walked because it is a dependency between two options and § 1's roster
  is all-optional by construction —
  `nvs_types::testing::check_retries_state_a_reason`.
- **`nvs test` reports one run in three formats**, one `Outcome` list behind
  all of them; `crates/nvs-cli/src/runner.rs`'s module doc is the home of why
  a verdict is decided once and where a machine format sends the program's
  own output.
- **`Core\Test` has eight assertion members plus `expectFailure`**;
  `crates/nvs-stdlib/src/test.rs`'s module doc is the home of why each subject
  is the type it is.
- What Stage 7 still owes is §§ 8-9 alone: `#[Fixture]` and data rows, which
  is what `nvs_types::testing::check_method_shape`'s parameter bullet waits
  on. The roster is at `crates/nvs-cli/src/runner.rs`'s own module doc.
- The conformance corpus is at **737**.

## Next group

**§ 8's `#[Fixture]`, which is the last thing between a `#[Test]` method and
its parameter list.** The file set is `crates/nvs-types/src/testing.rs` (the
option roster at `:129`, `TestCase` at `:142`, `check_method_shape` at `:215`
whose parameter bullet is deferred to exactly this, `check_payload` at `:291`)
plus `crates/nvs-cli/src/runner.rs` (`run_with_retries` at `:281`, `run_case`
at `:322`) and `crates/nvs-runtime/src/dispatch.rs:365`
(`construct_and_call`, whose parameter-declaring constructor is today a
catchable throw naming this very feature):

- [ ] **§ 8's `#[Fixture]` roster, built while checking** — ADR 0079 § 8
      (`docs/adr/0079-testing-is-a-language-feature.md:304`) makes a fixture
      built once in the parent and injected by parameter, so what is owed
      first is the table: which methods of a class declare `#[Fixture]`, at
      what return type, recorded beside `ExprTypeTable::tests` the way § 1's
      own table is, in the same walk. `check_class_tests` at
      `crates/nvs-types/src/testing.rs:158` is where it is collected.
- [ ] **§ 1's parameter bullet, refused or resolved** — a `#[Test]` parameter
      no fixture supplies is the fifth of § 1's compile errors and the one
      `check_method_shape` (`crates/nvs-types/src/testing.rs:215`) explicitly
      defers; with the roster in hand it is decided there, beside the three
      shape refusals it already makes, under `E0733` or a code of its own.
- [ ] **The runner's injection** — `construct_and_call`
      (`crates/nvs-runtime/src/dispatch.rs:365`) calls with no arguments
      today, and a parameter-declaring constructor is a catchable throw that
      names §§ 8-9 by hand; both ends move together, one `.nvst` over a
      fixture reaching two test methods.

## Backlog

- §§ 8-9's `#[Fixture]` and data rows — ADR 0079, the group above.
- § 2's isolate per test and parallelism — M5, `nvs_cli::runner`'s own doc.
- § 21's `nvs test --mutate` — ADR 0079 § 21, wants ADR 0018's coverage data.
- A `require` whose path is not a string literal runs nothing, silently —
  `nvs_hir::requires`' own known gap.
- `nvs_stdlib::debug` gap 1: an `array<T>` element and a shape literal's field
  carry no `secret` bit — ADR 0033's unmodelled container axis.
- `signatures.rs`' known gap: a class constant's declared type is unmodelled,
  so `Class::CONST` infers `mixed` at every expression site.
