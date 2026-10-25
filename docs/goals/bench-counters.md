---
looper: 1
title: "Benches report counted work that CI can gate on, and the probes and optimisation levels are tested configurations"
summary: "#[Bench] and nvs test --bench report statements, calls, allocations, bytes and GC cycles with wall time advisory, a blocking call counts the same on every run, a bench ramps an input fixed outside its program, CI runs the conformance suite with the probes on and once per optimisation level, every designed rule and gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs]
stages:
  - id: 1
    title: a GC cycle is counted, and a blocking call counts the same on every run
    status: open
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/tests/check_command.rs, tools/nv/proofs/perf.ts, data/gaps/nvs-cli/a-counted-run-under-a-ceiling-counts-by-its-cache-state.json]
    extra:
      context:
        rules: [testing/bench-counters, testing/member-perf-ledger, observability/gc-pause-is-its-own-event]
  - id: 2
    title: "`#[Bench]` and `nvs test --bench` report the counted rows"
    status: open
    context: [crates/nvs-types/src/derive.rs, crates/nvs-types/src/testing.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/bench-counters.md, docs/examples/lang/testing/chapter.md]
    extra:
      context:
        rules: [testing/bench-counters, testing/test-attribute, testing/report-formats, testing/tests-never-reach-a-build, testing/feature-proofs]
  - id: 3
    title: a bench ramps an input that is fixed outside its program
    status: open
    context: [tools/nv/cmd/scaling.ts, tools/nv/test/scaling.test.ts, data/gaps/nv/a-bench-cannot-ramp-an-input-fixed-outside-its-program.json]
    extra:
      context:
        rules: [testing/member-perf-ledger, testing/perf-two-mechanisms]
  - id: 4
    title: the conformance suite runs at each optimisation level
    status: open
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/cache.rs, .github/workflows/ci.yml, docs/rules/testing/probes-on-is-a-tested-configuration.md]
    extra:
      context:
        rules: [testing/probes-on-is-a-tested-configuration, testing/nvst-is-separate, config/the-extension-set-is-in-every-unit-key]
  - id: 5
    title: the conformance suite runs with the probes on
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-runtime/src/ctx/trace.rs, .github/workflows/ci.yml, docs/rules/testing/probes-on-is-a-tested-configuration.md]
    extra:
      context:
        rules: [testing/probes-on-is-a-tested-configuration, testing/debug-probes, testing/nvst-is-separate]
  - id: 6
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, bench-counters]
    reach: true
    floor: false
---

# Benches report counted work that CI can gate on, and the probes and optimisation levels are tested configurations

## Why

`nvs run --count` prints four counts that are the same on every machine (`crates/nvs-cli/src/main.rs:2263`),
and a program's author cannot use them: `#[Bench]` is not a recognized attribute
(`crates/nvs-types/src/derive.rs:73`), so a CI pipeline cannot gate on its own code's work. The counts
lack garbage-collection cycles (`crates/nvs-runtime/src/ctx/trace.rs:603`). Two gap records make
this repository's own benches noisy: a call on the blocking pool allocates a different amount from run
to run, and a bench cannot ramp an input that is fixed before its program starts. And the probes and
Cranelift's other optimisation levels are shapes that no test run takes
(`rule:testing/probes-on-is-a-tested-configuration`), which is where a JIT's instrumentation bugs live.

## Standing decisions

**A bench is test code that runs only under `--bench`.** `#[Bench]` marks a method in a class that
`nvs test` discovers. A plain `nvs test` skips it, and `nvs test --bench` runs the benches and no tests.
It is test code under `rule:testing/tests-never-reach-a-build`, so `test-surface`'s stage 1 keeps it out
of builds. **Gate for stage 2:** `docs/goals/test-surface.md` shows stage 1 `done` (or the file is gone)
and `docs/rules/testing/tests-never-reach-a-build.md` has no `**Not on disk.**` paragraph. `#[Bench]`
takes `skip` and `because`, at the types `#[Test]`'s roster gives them, and a bench runs once. Why: its
counts are the same on every run, so no repeat or iteration option buys anything (simplicity).

**The GC count joins the count line at its end.** `--count` prints `gc=<cycles>` after `bytes`, so
`tools/nv/proofs/perf.ts:49`, which matches from the start of the line, keeps reading the four it reads
today.

**Two `nvs test` flags select the tested configurations.** The user decided this. They are `--probes`,
which turns every probe bit on for each case, and `--opt-level none|speed`, which picks Cranelift's
optimisation level; stage 4's and stage 5's rule items record them. They are flags because the CI lane
runs the built binary, and a flag is visible in `nvs test --help` where an environment variable is not.
`--opt-level` replaces the `NOVIS_NO_OPTIMISE` variable the CI lane sets today
(`crates/nvs-codegen/src/lib.rs:896`). The compile cache keys on the level, so a unit compiled at one
level is never run at another.

**`--probes` waits on `trace-timeline`'s sink.** With `TRACE` on, the call events must go to a sink
and not grow in memory. **Gate for stage 5:** `docs/goals/trace-timeline.md` shows stage 6 `done` (or
the file is gone). If it does not hold, stage 4 runs first and stage 5 waits.

**A CI lane is proved locally by one conformance chapter.** The full suite at each level and with the
probes on runs only in CI. Locally, `nvs test --opt-level none tests/conformance/lang/` and
`nvs test --probes tests/conformance/lang/` stand for the lane, and the workflow change is checked by
reading it. Why: one chapter exercises codegen at the level without a full extra suite run per session.

**Trade-offs, all together.** Request path: nothing. A counted run does one more increment per
collection. Memory: one integer per run for the GC count, and nothing per bench beyond its row in the
report. Usability: a bench that gates in CI on every operating system, with wall time printed and
labelled advisory. Simplicity: one attribute and three flags. CI time grows by one conformance run per
added configuration.

**A blocking call frees its pool thread before it publishes its answer, and its slot stays per call.**
What moved the count from run to run was a second pool thread. The task submitted its next call before
the pool thread had counted itself idle, so `submit` started another thread and its allocations landed
in the task's count. `Queue::busy` now counts running jobs, and `run`'s job releases its thread before
it fills the slot (`crates/nvs-host/src/blocking.rs:124`). The slot `Arc` and the job box stay one each
per call. Both are fixed per call, so they count the same on every run, and a slot per task would add a
reuse protocol for no determinism gained (simplicity). Request path: one more lock per `run` job, on the
pool thread. Memory: nothing.

**Lanes.** Stage 3 touches only `tools/nv/` TypeScript and the bench `.nvs` files, so it runs as a side
lane in any order. Stage 1 (`crates/nvs-runtime/`, `crates/nvs-host/src/blocking.rs`) and stage 4
(`crates/nvs-codegen/`, `crates/nvs-cli/src/cache.rs`, `.github/workflows/ci.yml`) share only
`crates/nvs-cli/src/main.rs` and run in order. Stage 2 waits on `test-surface` stage 1 and stage 5 on
`trace-timeline` stage 6. Stages 4 and 5 share `crates/nvs-cli/src/main.rs` and the CI workflow, and run
in that order.

## Stages

### 1. a GC cycle is counted, and a blocking call counts the same on every run

**Does:** the counts gain GC cycles, and a call run on the blocking pool allocates the same amount
however its job ends.

- [x] `Counted` gains a GC cycle count (`rule:testing/bench-counters`). `crates/nvs-runtime/src/ctx/trace.rs:725`
- [x] The count is raised where a collection runs. `crates/nvs-runtime/src/ctx/safepoint.rs:395`
- [x] `--count` prints `gc=<cycles>` last on its line. `crates/nvs-cli/src/main.rs:2538`
- [x] The blocking pool's allocations for a call do not depend on when the answer lands: a call frees its thread before it publishes its answer, so calls one after another use one thread (`## Standing decisions`). `crates/nvs-host/src/blocking.rs:124`, `crates/nvs-host/src/blocking.rs:995`
- [x] The `writeStdin` bench drops its gap marker and judges the same on repeated runs, and the gap record is deleted (`rule:testing/member-perf-ledger`). `benches/members/core/Process-Handle/writeStdin.nvs:5`
- [x] A Rust test that two counted runs of a program with a collection print the same line (`rule:testing/bench-counters`). `crates/nvs-cli/tests/check_command.rs:331`
- [ ] A counted run that collects under a ceiling prints the same line cold and warm, the test above compares one of each, and the gap record is deleted (`rule:testing/bench-counters`). `data/gaps/nvs-cli/a-counted-run-under-a-ceiling-counts-by-its-cache-state.json:1`, `crates/nvs-runtime/src/ctx/limits.rs:281`

### 2. `#[Bench]` and `nvs test --bench` report the counted rows

**Does:** a recognized attribute, and a flag that runs benches and reports their counts.

**Gate:** `test-surface` stage 1, as `## Standing decisions` names it.

**Split:** once the attribute and flag land, one writing subagent writes `about.md`, the three examples,
the Novis test, the attack and the bench at the paths `bun nv proofs --id` prints, under § *Text an end
user reads*. The session keeps the Rust tests and the rule flip.

- [ ] The rule settles the attribute's options and what `--bench` runs, as the standing decision says (`rule:testing/bench-counters`). `docs/rules/testing/bench-counters.md:17`
- [ ] `#[Bench]` joins the recognized roster with its card, and its methods are collected beside `#[Test]`'s (`rule:testing/test-attribute`). `crates/nvs-types/src/derive.rs:73`, `crates/nvs-types/src/testing.rs:231`
- [ ] `--bench` runs each bench under the counting mode and reports statements, calls, allocations, bytes and GC cycles, with wall time labelled advisory, in the text report and the JSON report, with its help text (`rule:testing/report-formats`). `crates/nvs-cli/src/runner.rs:142`, `crates/nvs-cli/src/runner.rs:2830`
- [ ] A Rust test that a bench's rows are identical across two runs (`rule:testing/bench-counters`). `crates/nvs-cli/src/runner.rs:3458`
- [ ] A heading for the feature in the testing chapter, and its feature proofs, with `bun nv proofs --id` on its id passing (`rule:testing/feature-proofs`). `docs/examples/lang/testing/chapter.md:15`, `docs/examples/lang/testing/bench-and-nvs-test-bench/about.md:1`
- [ ] The rule `shipped`, its `**Not on disk.**` paragraph gone. `data/rules/testing/bench-counters.json:3`, `docs/rules/testing/bench-counters.md:17`

### 3. a bench ramps an input that is fixed outside its program

**Does:** a sibling form whose size sets a request body, standard input, the environment or a set of
generated classes.

**Split:** once the sibling form lands, writing subagents, one after another, write the siblings in the
four batches below. Each removes its benches' `// proof: gap nv/a-bench-cannot-ramp-an-input-fixed-outside-its-program`
line. The session checks each batch and ticks it.

- [ ] A sibling form whose `N` sizes the input the program reads, such as a `.nvsr` or `.in` the ramp writes at each size (`rule:testing/member-perf-ledger`). `tools/nv/cmd/scaling.ts:919`
- [ ] A test of the new form beside the existing ones (`rule:testing/perf-two-mechanisms`). `tools/nv/test/scaling.test.ts:58`
- [ ] Batch 1, a request body: `Request/body`, `bodyStream`, `bytes`, `files`, `headers`, `json`, `jsonAs`, `postAs`, `queryAs`. `benches/members/core/Request/body.nvs:1`
- [ ] Batch 2, a request part and a signed route: `Request-Part/content`, `readAll`, `saveTo`, `Router/signedRoute`. `benches/members/core/Request-Part/content.nvs:1`
- [ ] Batch 3, standard input, the environment and the command line: `IO/stdin`, `Env/all`, `Command/completions`, `help`, `run`. `benches/members/core/IO/stdin.nvs:1`
- [ ] Batch 4, generated classes: the two `lang/attributes/core-program-implementing*` benches. `benches/members/lang/attributes/core-program-implementing-i-every-class-that-is-an-i.nvs:1`
- [ ] No bench carries the marker, and the gap record is deleted. `data/gaps/nv/a-bench-cannot-ramp-an-input-fixed-outside-its-program.json:1`

### 4. the conformance suite runs at each optimisation level

**Does:** `nvs test --opt-level`, and the CI lane uses it.

- [ ] The rule names `--opt-level`, as the standing decision says (`rule:testing/probes-on-is-a-tested-configuration`). `docs/rules/testing/probes-on-is-a-tested-configuration.md:18`
- [ ] Cranelift's level is a parameter of the host ISA in place of `NOVIS_NO_OPTIMISE`, and the compile cache keys on it (`rule:config/the-extension-set-is-in-every-unit-key`). `crates/nvs-codegen/src/lib.rs:896`, `crates/nvs-codegen/src/lib.rs:2076`, `crates/nvs-cli/src/cache.rs:1291`
- [ ] `--opt-level` on `nvs test`, with its help text. `crates/nvs-cli/src/runner.rs:142`
- [ ] A Rust test that a unit cached at one level is not loaded at the other (`rule:config/the-extension-set-is-in-every-unit-key`). `crates/nvs-cli/src/cache.rs:1585`
- [ ] The CI lane runs `--opt-level none` in place of the variable; proved locally on `tests/conformance/lang/`. `.github/workflows/ci.yml:229`

### 5. the conformance suite runs with the probes on

**Does:** `nvs test --probes`, and a CI lane that uses it.

**Gate:** `trace-timeline` stage 6, as `## Standing decisions` names it.

- [ ] The rule names `--probes` (`rule:testing/probes-on-is-a-tested-configuration`). `docs/rules/testing/probes-on-is-a-tested-configuration.md:18`
- [ ] Over a `.nvst` tree, `--probes` runs each case with every probe bit on and its data discarded, and output is compared as usual (`rule:testing/debug-probes`). `crates/nvs-cli/src/main.rs:2377`, `crates/nvs-cli/src/main.rs:2433`
- [ ] A Rust test that a case passes with and without `--probes` and the probe data is discarded (`rule:testing/nvst-is-separate`). `crates/nvs-cli/src/main.rs:2377`
- [ ] CI runs the conformance suite once with the probes on, beside the existing run; proved locally on `tests/conformance/lang/`. `.github/workflows/ci.yml:228`
- [ ] The rule `shipped`, its `**Not on disk.**` paragraph gone, and `bun nv owners --closes bench-counters` passes. `data/rules/testing/probes-on-is-a-tested-configuration.json:3`, `docs/rules/testing/probes-on-is-a-tested-configuration.md:18`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stage 1 has one item left. `nvs run --count` prints `gc=<cycles>` last
(`crates/nvs-runtime/src/ctx/safepoint.rs:395`, `crates/nvs-cli/src/main.rs:2538`). The blocking pool
frees a `run` job's thread before its answer is published, so `writeStdin` counts the same on every run.
Its marker and gap record are gone. On Windows, 6 runs of an 8192-round copy differed by one thread start
(5 allocations) before the fix. After it, 1 run in 37 at 8192 differed by one allocation of 72 bytes, and
25 runs at 4096 (the ramp's max) all matched. `bun nv scaling --growth` judged both of its benches `flat`
three times out of three. Linux, where `cat` reads the input, was not run locally.

The new item comes from a finding of this session. A counted run that collects under a ceiling prints one
line every cold run and another every warm run: 84 more allocations and 3444 more bytes cold, with the
same `gc=14`. With no ceiling the two match. The gap record states it.

Tests expected to go red: none. `two_counted_runs_with_a_collection_print_the_same_line` and
`calls_one_after_another_use_one_thread_and_allocate_the_same` are new and pass. The second fails with the
early release taken out.

The nightly run `40075586234` was on `3ea6f1550`, and HEAD already fixes each job it failed. `crypto-bigint`
is 0.7.6, `cargo fmt --check` is clean, and the three miri tests are handled by `c953c3456`, `220cb82b7`
and `c4842ec8f`. `env-secret-proofs` passes when run by hand.

### Next

**Stage 1's last item: a ceiling that counts the same cold and warm**, file set
`crates/nvs-runtime/src/ctx/limits.rs`, `crates/nvs-runtime/src/budget.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-cli/src/cache.rs`, `crates/nvs-cli/tests/check_command.rs`. No gate.

- [ ] Find what the cold compile moves on the run's thread: the ceiling is `memory_base` plus the limit, so a pre-run allocation freed during the run, or a unit held at a different size after a compile than after a cache load, moves where it is crossed. `crates/nvs-runtime/src/ctx/limits.rs:290`, `crates/nvs-cli/src/cache.rs:1637`
- [ ] Fix the cause so the crossings depend only on what the program allocated; reproduce with the test's program under `[limits] memory = "2M"`, one fresh `--data` run cold and then warm. `crates/nvs-runtime/src/ctx/limits.rs:290`
- [ ] The test compares one cold run with one warm run, and the gap record is deleted. `crates/nvs-cli/tests/check_command.rs:331`, `data/gaps/nvs-cli/a-counted-run-under-a-ceiling-counts-by-its-cache-state.json:1`
- [ ] Stage 1's status is `done` in the front matter. `docs/goals/bench-counters.md:9`

### Backlog

- Stage 3 is a side lane any session may take (`tools/nv/cmd/scaling.ts`).
- Stage 4 follows stage 1 (`crates/nvs-codegen/src/lib.rs:896`).
- The gate stage watches the nightly `writeStdin` perf proof on Linux for the leftover one-allocation
  difference seen above the ramp's max.
