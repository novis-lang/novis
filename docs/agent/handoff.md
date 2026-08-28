# Handoff

## State

**M4's Stage 7: ADR 0079 § 4's roster is closed but for `assertThrows`.**
`python tools/loop.py --list` reports no named `.nvst` case owed by any stage.

- **`Core\Test` has six assertion members plus `expectFailure`.**
  `assertTrue` (declared `bool`), `assertNull` (`mixed`) and `assertCount`
  (`array<T>` plus a `uint`, `Core\Arr::count`'s own signature) landed beside
  the three equality ones; `crates/nvs-stdlib/src/test.rs`'s module doc is the
  home of why each subject is the type it is.
- **What is owed is `assertThrows` and § 20's `assertDoesNotThrow`**
  (`nvs_stdlib::test`'s known gap 3). They are the one shape on § 4's roster
  that judges a `callable`'s outcome rather than a value, and the empty-ledger
  message in `nvs_cli::runner` names the rule rather than `assertDoesNotThrow`
  until it resolves.
- The conformance corpus is at **732**.

## Next group

**`assertThrows` and § 20's `assertDoesNotThrow`.** The file set is
`crates/nvs-stdlib/src/test.rs` (rows at `:106`, `address` at `:168`, members
below `:184`, the `failed`/`held` pair the ledger half is already written in)
plus whatever accessor the first slice needs on `crates/nvs-runtime/src/ctx.rs`,
and one new case beside
`tests/conformance/core/a-predicate-assertion-judges-the-one-subject-its-type-admits.nvst`:

- [ ] **`assertDoesNotThrow(callable $body, {message?: string})`** (ADR 0079
      § 20) — the cheaper half and independent of the class question: run the
      body through `nvs_runtime::call_closure` as
      `crates/nvs-stdlib/src/test.rs:346`'s `expectFailure` already does, fail
      naming the pending message where it threw, and consume that throw with
      `ctx.take_pending()` so the assertion's own verdict is what propagates.
      It is also § 20's way out of the empty-ledger rule, so
      `crates/nvs-cli/src/runner.rs`'s message can name the member once it
      resolves.
- [ ] **`assertThrows(callable $body, string $expected, {message?: string})`**
      (ADR 0079 § 4) — `ParseError::class` folds to the fully qualified name as
      a `string` constant, so the match is by name. The scouting is done: a
      pending failure is one of two shapes
      (`crates/nvs-runtime/src/ctx.rs:449`'s `Pending`), a helper-raised one
      carrying a `ThrownClass` whose name is
      `crates/nvs-runtime/src/throwable.rs:132`, and a program-raised one
      carrying a `Thrown` object whose ancestry test is
      `crates/nvs-runtime/src/object.rs:1275`'s `is_instance_of` — which takes a
      `*const ClassDesc` and so wants either a name→descriptor lookup on the
      class table or a walk of the descriptor's own flattened ancestor names.
      **Decide which and record it at the new `Ctx` accessor**; matching a
      subclass is what PHP's own `expectException` does and is the safe
      reading.
- [ ] **One `.nvst` over both members**, in `tests/conformance/core/` —
      the thrown class matched exactly and through a parent, the wrong class,
      the body that threw nothing, and `assertDoesNotThrow` over a body that
      throws, with the count of agreements rather than a line read off each.

## Backlog

- `Core\Test\Failure`'s own class is not what `assertThrows` should match by
  accident — a failed assertion inside the body is a failure, not the expected
  throw (`docs/adr/0079-testing-is-a-language-feature.md` § 5).
- § 4's non-`Comparable` refusal under `assertEquals` is a runtime throw and
  belongs in `nvs_types` (`nvs_stdlib::test`'s known gap 1).
- § 2's isolate-per-test and parallelism (M5), § 20's `retries:`/`FLAKY` and
  § 22's `--format=junit|json` (`nvs_cli::runner`'s own list).
- A `require` whose path is not a string literal runs nothing at all, silently
  (`nvs_hir::requires`' own known gap).
- ADR 0033's container axis: an `array<T>` element and a shape literal's field
  carry no `secret` bit (`nvs_stdlib::debug`'s known gap 1).
