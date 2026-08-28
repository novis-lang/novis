# Handoff

## State

**M4's Stage 7: § 8's `#[Fixture]` roster is built, and what Stage 7 still
owes is that section's injection plus § 9's data rows.** `python
tools/loop.py --list` reports no named `.nvst` case owed by any stage.

- **A `#[Fixture]` roster is a second table beside § 1's**, keyed by the type
  each row supplies; `crates/nvs-types/src/testing.rs`'s module doc is the
  home of why the two are collected in one walk and why the fixture shape
  rules are the inverse of a test's. ADR 0079 § 8 carries the rule.
- **Four fixture declarations are `E0735`** — not `static`, not `public`,
  returning `void`, or carrying `#[Test]` as well — and two fixtures of one
  type are `E_DUPLICATE_DECLARATION`, § 8 resolving by type.
- **A `#[Test]` method with a parameter is still accepted and still cannot
  run**: the runner calls it with no arguments. Resolving a parameter against
  the roster and injecting the value are **one** slice, not two — accepting a
  resolvable parameter before the injection exists opens the hole it looks
  like it closes, and refusing every parameter would refuse § 8's own
  worked example.
- **A retried test is flaky, in all three formats at once.**
  `Outcome::Flaky` is a fifth verdict carrying its attempt count and the last
  failed attempt's failures; `crates/nvs-cli/src/runner.rs`'s
  `run_with_retries` is the home of why only a failure is retried.
- **`nvs test` reports one run in three formats**, one `Outcome` list behind
  all of them; `crates/nvs-cli/src/runner.rs`'s module doc is the home of why
  a verdict is decided once.
- **`Core\Test` has eight assertion members plus `expectFailure`**;
  `crates/nvs-stdlib/src/test.rs`'s module doc is the home of why each subject
  is the type it is.
- The conformance corpus is at **738**.

## Next group

**§ 8's injection, which is the whole of what stands between the roster and a
test that takes a parameter.** The file set is
`crates/nvs-types/src/testing.rs` (the roster at `:183`, `check_class_tests`
at `:206`, `check_method_shape` at `:399` whose parameter bullet waits on
this) plus `crates/nvs-cli/src/runner.rs` (`run_case` at `:343`,
`run_with_retries` at `:281`) and
`crates/nvs-runtime/src/dispatch.rs:365` (`construct_and_call`, which is
where an argument list would be handed over and whose parameter-declaring
constructor is today a catchable throw naming this feature):

- [ ] **§ 1's parameter bullet, resolved against the roster** — ADR 0079 § 8
      (`docs/adr/0079-testing-is-a-language-feature.md:304`) resolves a
      `#[Test]` parameter **by type**, so each one is matched against the
      class's `ExprTypeTable::fixtures` row and a parameter no row supplies is
      refused where it is written, naming the type it asked for. Record the
      resolution on `TestCase` (`crates/nvs-types/src/testing.rs:170`) so the
      runner reads an order rather than re-deriving one. A fixture's *own*
      fixture parameters resolve the same way and a cycle is a compile error,
      which is that section's last sentence.
- [ ] **The runner builds each fixture once and passes the values in** —
      one call per row before the class's tests run, the value held for the
      whole class, handed to `construct_and_call` as an argument list. § 8's
      graph copy is `spawn`'s and is M5, so a value shared within one process
      is the safe reading to land now and to say out loud at
      `nvs_cli::runner`.
- [ ] **The `.nvst` that pins both** — § 8's own worked example running, the
      fixture built once for two tests counted rather than read off a line,
      and the unsatisfiable parameter as a reject twin.

## Backlog

- § 9's data rows, the other half of what Stage 7 owes — ADR 0079 § 9.
- § 2's isolate-per-test and parallelism (M5) — `crates/nvs-cli/src/runner.rs`.
- `#[Fixture]` on a property or a const is not checked for a payload, only on
  a method — `crates/nvs-types/src/testing.rs`.
- A `require` whose path is not a string literal runs nothing, silently —
  `nvs_hir::requires`' own known gap.
- ADR 0024 § 5's `string as Core\Html\Markup` waits on `Core\Html` (M7) —
  `nvs-ir`'s own catch-all.
- `signatures.rs` leaves a class constant's declared type unmodelled —
  that module's own known gap.
