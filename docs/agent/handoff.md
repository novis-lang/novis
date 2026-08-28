# Handoff

## State

**M4's Stage 7: ADR 0079 §§ 1, 4, 5, 20, 22's human half and 23 are closed,
and every named `.nvst` case each stage owes has landed** — `python
tools/loop.py --list` reports none.

- **`--RUN--` is how a `.nvst` case names its subcommand**: one line holding
  `run` (the default) or `test`, a closed roster, applying to `--FILE--` alone.
  `crates/nvs-test`'s module doc is the format's one home and states it.
- **`nvs_cli::runner` is the runner** and its module doc is the home of what it
  owes and of the two decisions it took: the entry file's own top-level
  statements do not run, and class order is the roster's sorted order.
- **ADR 0079 § *Verification* now names what pins each of its bullets**, and
  the three claims only a Rust test can assert.
- § 4's roster is still the three equality members.
  `assertTrue`/`assertNull`/`assertCount`/`assertThrows` are owed
  (`nvs_stdlib::test`'s known gap 3), and § 20's `assertDoesNotThrow` joins
  them — the empty-ledger failure names the rule rather than that member
  because it does not resolve yet. That is the next group.
- The conformance corpus is at **731**.

## Next group

**Closing ADR 0079 § 4's roster.** The file set is `crates/nvs-stdlib/src/`
(`test.rs` plus its one registry line) and one new case:

- [ ] **`assertTrue`/`assertNull`/`assertCount`** (ADR 0079 § 4) — three rows
      beside the three that exist, `crates/nvs-stdlib/src/test.rs:106` being
      the roster and `:159` the members below it, with
      `crates/nvs-stdlib/src/registry.rs:748` the one line that registers the
      class. Each funnels through `test.rs:303`'s `failed` and `:321`'s `held`,
      so the ledger half is already written; `assertCount` wants a subject the
      spec gives a length to, which is where the `CoreTy` row needs deciding.
- [ ] **`assertThrows` and § 20's `assertDoesNotThrow`** (ADR 0079 §§ 4, 20) —
      both take a `callable` body, so both reach `nvs_runtime::call_closure`
      the way `Core\Test::expectFailure` already does; `assertDoesNotThrow` is
      the stated way out of the empty-ledger rule, so `nvs_cli::runner`'s
      empty-ledger message stops naming the rule and names the member.
- [ ] **One `.nvst` over the new members**, in `tests/conformance/core/`
      beside `an-assertion-compares-its-subject-against-its-expectation.nvst`,
      asserting that the five agree on what they record in the ledger rather
      than what each printed.

## Backlog

- §§ 8-9's `#[Fixture]` injection, which is what makes § 1's
  unsatisfiable-parameter compile error decidable at all
  (`nvs_types::testing::check_method_shape`).
- § 22's `--format=junit`/`--format=json` and § 20's `retries:`/`FLAKY`
  (`nvs_cli::runner`'s "what is owed").
- § 2's isolate-per-test and parallelism, which are M5.
- A `require` whose path is not a string literal runs nothing at all, silently
  (`nvs_hir::requires`' own known gap).
- ADR 0033's unmodelled container axis: an `array<T>` element and an ADR 0036
  shape field carry no `secret` bit (`nvs_stdlib::debug`'s known gap 1).
