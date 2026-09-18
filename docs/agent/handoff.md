# Handoff

## State

**Goal `test-doubles`, stage 5 is complete and green** — `python tools/verify.py` 12 of 12.
`Core\Test::assertCompletes` is a registry row with a body (`crates/nvs-stdlib/src/test.rs:773`),
five edits done and the `§13 Test::assertCompletes` ratchet key struck in the same slice, so
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt` now holds `Core\BigInt`
alone.

**The budget is a shape, not a bag.** `{within: Duration}` is `CoreTy::Shape(WITHIN)` because a
bag's option is optional by construction and there is no duration this member could pick for a
caller — that keeps ADR 0079 § 16's own `{within: Duration::millis(50)}` spelling and makes the
field required. The ABI slots are named once at `crates/nvs-stdlib/src/test.rs:355`.

**The clock moves before `$body` runs**, so a retry or a backoff inside it measures against a clock
already granted the budget; the helper's doc comment owns that reasoning. What "still running" means
is `nvs_host::children_still_running` against a count taken before the call — the same counter the
runner reads at the test's own boundary.

**Both sides are guarded.** Three `.nvst` cases pin the refusal (the only side a case can reach, a
case never being inside a `#[Test]`), and `crates/nvs-cli/src/runner.rs:2849` is the accepted side
over the fixture `crates/nvs-cli/tests/fixtures/runner/assert-completes.nvs`.

## Next group

**Stage 6: the proofs and the rulebook** — one file set: `docs/rules/testing.json`,
`tests/conformance/reject/`, `tests/conformance/core/`.

- [ ] **ADR 0079 § *Verification*'s § 10 bullets as conformance cases** — goal prose stage 6. A
      double missing a method and one declaring a method the interface lacks, each an
      `--EXPECTF-ERROR--` case under `tests/conformance/reject/`, beside the reference case
      `tests/conformance/reject/a-method-reference-names-a-method-the-interface-lacks.nvst:1`; and a
      double satisfying an interface passed to a parameter of that type under
      `tests/conformance/core/`. `rule:testing/doubles`.
- [ ] **`rule:testing/doubles` flipped to `shipped`** — its entry is
      `docs/rules/testing.json:142`, its empty `guardedBy` is `docs/rules/testing.json:146`, and the
      cases the item above writes are what fills it. Then `python tools/rules.py --render`.
- [ ] **`rule:testing/task-tree-and-virtual-clock` gains its new guards** — the same
      `docs/rules/testing.json:142` block, one entry further down: the three
      `tests/conformance/core/assert-completes-*.nvst` cases and the fixture behind
      `crates/nvs-cli/src/runner.rs:2849`. Re-render in the same call as the item above.

## Backlog

- `Core\BigInt` is the one key left in
  `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:24` — goal `bigint`, not
  this one.
- A `partial` delegating what it does not override is listed in stage 6's prose and has no case yet
  — `docs/agent/loop-goal.md:135`.
- Mutation testing is M10's, not this goal's — `docs/plan/m10.md`.
