# Handoff

## State

**Stage 0b's residue is closed: ADR 0063 R2 now holds end to end, including for a parameter whose
spec name is a reserved word.** `crates/nvs-syntax/src/parser/expr.rs`'s `parse_arg` reads any word
before a `:` as an argument name, keyword or not, so `Core\Arr::map(fn: …, a: …)` parses and the
seven `fn`-named rows need no rename. The plan's `Open now` is the one home for what landed; the rule
itself lives in `parse_arg`'s doc comment.

**Stage 7 is the open front, and its acceptance check is now aimed correctly.** The
`7 test isolates` check in `docs/agent/loop-goal.toml` runs `-p nvs-cli`, not `-p nvs-test`, and
`docs/agent/goals/2-concurrency.toml` is byte-identical again. All four of its names are still
unwritten — that is item 25 itself, and it is the next group below.

**Orientation gap, unchanged from last session:** `[context] modules` has no pattern for
`crates/nvs-cli/src/runner.rs`, which the whole next group is written against, nor for
`crates/nvs-syntax/src/parser/expr.rs`. Add both selectors before the next session opens this group.

## Next group

**All three slices share `crates/nvs-cli/src/runner.rs` and `crates/nvs-host/src/isolate.rs`; they
are Stage 7 item 25 and ADR 0079 §§ 2 and 16, and together they turn the goal's `7 test isolates`
check green.**

- [ ] **One isolate per `#[Test]`.** `crates/nvs-cli/src/runner.rs:190` builds one `Ctx` for the
      whole run and `:436` is the per-test entry; `:48` and `:299` are the two comments still saying
      isolate-per-test is M5's and unbuilt. `crates/nvs-host/src/isolate.rs:90` is `Isolate`, whose
      program arrives as a closure rather than a path, so the runner — which already holds the
      compiled unit — is the one place that closure can be built. The guard is
      `each_test_runs_in_its_own_isolate_sharing_only_compiled_code`, and what it must observe is a
      static written by one test that the next one does not read back (ADR 0116 § 4's fresh statics
      base).
- [ ] **The runner owns the test's task tree.** Same file: a test's isolate is a child task, so a
      `Core\Task::all` inside a `#[Test]` has a calling task to be a child of, and nothing the test
      started is still running when its row is reported (ADR 0072 § 4). Guard:
      `the_runner_owns_the_tests_task_tree`.
- [ ] **A fixed clock and a seed are per-test.** Guards `a_test_at_a_fixed_clock_reads_that_clock`
      and `a_test_with_a_seed_draws_the_same_sequence_twice`; both are properties of the child
      context the slice above builds, so they are cheap once it lands and expensive before it.

## Backlog

- Item 22's `Core\Script::valueOrThrow` is still unwritten — `docs/plan/m5.md`, Stage 6.
- `Core\Secret::reveal()` is absent from the registry, so ADR 0033's escape hatch is open at both
  ends — `crates/nvs-stdlib/src/registry.rs`.
- The graph copy asks its class-identity question of the answer but not of the argument; closing it
  means the `nvs_runtime::script` seam answering with a unit's class table
  (`crates/nvs-runtime/src/graph.rs` module doc).
- M4's 1000-case conformance floor — `docs/implementation-plan.md`, Stage 8.
- The other 342 registry rows carry no ADR 0117 reference card — `crates/nvs-stdlib/src/registry.rs`.
- `python tools/check-migration.py` reads 34% classified against a 100% floor — Stage 8.
