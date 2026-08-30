# Handoff

## State

**Every `#[Test]` runs in an isolate of its own (ADR 0079 § 2), and the first of Stage 7's four
names is green.** `crates/nvs-cli/src/runner.rs`'s `run_in_isolate` builds one
`nvs_host::Isolate` per reported case; its module doc § *Every test is its own isolate* is that
design's one home, and the guard is
`each_test_runs_in_its_own_isolate_sharing_only_compiled_code`. Two decisions the item did not
predict, both recorded in the code that makes them: the isolate is per **test**, not per attempt,
so `run_with_retries` runs on the child's stack and § 20's retry stays usable; and § 8's fixtures
are built once per class in the parent and **copied** in per test through
`nvs_runtime::CrossedFixtures`. Each cost one conformance case its old spelling — a `static`
written on one side of a test boundary and read on the other is exactly what § 2 forbids — so both
now observe through the value a test is handed. `tools/leak-check.sh --test` over a fixture
exercising the new retain/copy/release edge reports no definite loss.

**Stage 7's other three names are unwritten and are the next group.** They are item 25's rest:
the task tree (§ 16), and the fixed clock and the seed (§ 12). None of them has a line of code
yet.

**Orientation gap, unchanged:** `[context] modules` has no pattern for `crates/nvs-cli/src/`,
which the whole of Stage 7 is written against — `runner.rs` most of all — so the map block prints
nothing for the file every remaining slice edits. Add that selector.

## Next group

**All three slices share `crates/nvs-cli/src/runner.rs` and `crates/nvs-host/src/isolate.rs`;
they are Stage 7 item 25's remainder and ADR 0079 §§ 12 and 16, and together they turn the goal's
`7 test isolates` check green.**

- [ ] **The runner owns the test's task tree.** `crates/nvs-cli/src/runner.rs:632` is the
      `nvs_host::Isolate::new(...).run(ctx)` that today takes `nvs_host::isolate.rs:189`'s
      no-scheduler path, because `nvs test` installs none — `Wake::current()` is `None`, so the
      child runs on the caller's stack and a `Core\Task::spawn` inside a test has no calling task
      to be a child of. `nvs run` already does the other thing (`crates/nvs-cli/src/main.rs`,
      § *What `run` executes*): one `Scheduler`, a reactor over it, `TaskRoot::Request`. Do the
      same in `run` (`runner.rs:188`) so the whole suite runs inside one task. § 16's second half
      is the guard's own claim — **a task still running when the test returns is a failure, named
      as such** — and `nvs_host::group`'s "nothing still running when the call returns" is the
      mechanism to read it off. Guard: `the_runner_owns_the_tests_task_tree`.
- [ ] **A fixed clock is per test.** § 12 puts the clock under the test's control and § 16 needs
      it: `#[Test(at: "2026-01-01T00:00:00Z")]`. The option reaches the runner as a
      `nvs_types::testing::TestCase::options` entry, read exactly as `retry_allowance`
      (`runner.rs:496`) and `skip_reason` read theirs; where it lands is a word on the child's own
      `Ctx`, which is the same shape ADR 0116 § 4's statics base has. Guard:
      `a_test_at_a_fixed_clock_reads_that_clock`.
- [ ] **A seed is per test.** The sibling option and the same three edits, over whatever
      `Core\Random` reads. Guard: `a_test_with_a_seed_draws_the_same_sequence_twice`.

## Backlog

- The `errors` cards that disagree with the spec's § 10 tree — read from the helper bodies during
  the ADR 0117 backfill, listed in `docs/adr/0117-*.md` § 1's own note. Unresolved.
- § 2's **parallelism**: the isolates are built and joined one at a time
  (`runner.rs`'s module doc § *What is owed*). It is a scheduling question and it waits on the
  task tree above.
- ADR 0006's `$result->valueOrThrow()` is owed as `Core\Script::valueOrThrow($result)` — item 22,
  `docs/plan/m5.md`.
- `Core\Secret::reveal()` is still unregistered, so ADR 0033's way out of a `secret` boundary
  refusal is open at both ends — item 18.
- The isolate boundary asks ADR 0023 § 2's unresolvable-class question of the answer and not of
  the argument (`crates/nvs-host/src/isolate.rs`'s module doc, the known gap).
