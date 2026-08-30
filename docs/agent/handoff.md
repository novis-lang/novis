# Handoff

## State

**Stage 7 item 25's second name is green: the runner owns the test's task tree (ADR 0079 § 16).**
`crates/nvs-cli/src/runner.rs:316`'s `run_suite_in_a_task` installs a `Scheduler`, a reactor and
the isolate resolver and spawns the whole suite as one `TaskRoot::Request` task — the same three
installations `nvs run` makes — so each test's isolate is a *child* task and a `Core\Task::all`
inside a test has a calling task to be a parent (it reached `Fault::fatal` before, no host being
installed). § 16's second half is `nvs_host::children_still_running()`, read inside
`run_in_isolate`'s program closure the moment the test body returns, folded in by
`Outcome::with_tasks_left_running`: a leftover task is a failure that names itself instead of
being cancelled silently at the retire. The module doc § *The runner owns the test's task tree*
is that design's one home. Guard:
`the_runner_owns_the_tests_task_tree`, over
`crates/nvs-cli/tests/fixtures/runner/task-tree.nvs` — one test whose `Task::all` only has
children because the suite is a task, one that leaves a `spawn script` unawaited.

**Stage 7's other two names are unwritten and are the next group**: § 12's fixed clock and its
seed. Neither has a line of code, but the checker already accepts both option keys —
`crates/nvs-types/src/testing.rs:233-238` lists `at` and `seed` beside `skip` and `retries` — so
what is missing is the runner reading them and the isolate honouring them.

**Orientation gap, unchanged:** `[context] modules` has no pattern for `crates/nvs-cli/src/`,
which the whole of Stage 7 is written against, so the map block prints nothing for `runner.rs` —
the file every remaining slice edits. Add that selector.

## Next group

**Both slices share `crates/nvs-cli/src/runner.rs`, `crates/nvs-types/src/testing.rs` and
`crates/nvs-stdlib/src/time.rs`; they are ADR 0079 § 12 and item 25's remainder, and together
they turn the goal's `7 test isolates` check green.**

- [ ] **A fixed clock is per test.** `#[Test(at: "2026-01-01T00:00:00Z")]` — the key is already
      in the checker's option table (`crates/nvs-types/src/testing.rs:233`, `("at",
      OptionTy::Str)`), so the work is on the runtime side: read it beside `retry_allowance`
      (`crates/nvs-cli/src/runner.rs:620`) and `skip_reason` (`:936`), which are the two worked
      examples of reading an option off a `TestCase`; carry it onto the isolate's own context in
      `run_in_isolate` (`:649`), where the child's `Ctx` is the only thing a test can observe;
      and make `Core\Time::now` (`crates/nvs-stdlib/src/time.rs:3149`) read it. § 12's
      `Core\Test::advance(Duration)` is the mutator and is a new `Core` member — five edits, per
      conventions.md. Guard: `a_test_at_a_fixed_clock_reads_that_clock`.
- [ ] **A seed is per test.** `("seed", OptionTy::Int)` at `crates/nvs-types/src/testing.rs:235`,
      the same three edits over `Core\Random`/`Core\Uuid` instead of `Core\Time`, and § 12's
      point is that the *same* declaration makes both deterministic. Guard:
      `a_test_with_a_seed_draws_the_same_sequence_twice`.

## Backlog

- § 2's parallelism: isolates are made and joined one at a time (`runner.rs`'s § *What is owed*).
- A `spawn script` inside a test resolves relative to the working directory, so the guard's
  fixture writes a path from `crates/nvs-cli` — `crates/nvs-cli/src/script.rs`'s module doc owns
  the anchoring rule.
- The spec-versus-code disagreements ADR 0117's card backfill turned up (docs/agent/handoff.md's
  predecessor listed them; `python tools/check-migration.py --report`).
- M4's 1000-case corpus count, met as the suite grows (`docs/plan/m4.md`).
