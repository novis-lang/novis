---
looper: 1
title: "Benches report counted work that CI can gate on, and the probes and optimisation levels are tested configurations"
summary: "#[Bench] and nvs test --bench report statements, calls, allocations, bytes and GC cycles with wall time advisory, a blocking call counts the same on every run, a bench ramps an input fixed outside its program, CI runs the conformance suite with the probes on and once per optimisation level, every designed rule and gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs]
stages:
  - id: 1
    title: a GC cycle is counted, and a blocking call counts the same on every run
    status: done
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/tests/check_command.rs, tools/nv/proofs/perf.ts]
    extra:
      context:
        rules: [testing/bench-counters, testing/member-perf-ledger, observability/gc-pause-is-its-own-event]
  - id: 2
    title: "`#[Bench]` and `nvs test --bench` report the counted rows"
    status: done
    context: [crates/nvs-types/src/derive.rs, crates/nvs-types/src/testing.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/bench-counters.md, docs/examples/lang/testing/chapter.md]
    extra:
      context:
        rules: [testing/bench-counters, testing/test-attribute, testing/report-formats, testing/tests-never-reach-a-build, testing/feature-proofs]
  - id: 3
    title: a bench ramps an input that is fixed outside its program
    status: done
    context: [tools/nv/cmd/scaling.ts, tools/nv/test/scaling.test.ts, benches/members/README.md]
    extra:
      context:
        rules: [testing/member-perf-ledger, testing/perf-two-mechanisms]
  - id: 4
    title: the conformance suite runs at each optimisation level
    status: done
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/cache.rs, .github/workflows/ci.yml, docs/rules/testing/probes-on-is-a-tested-configuration.md]
    extra:
      context:
        rules: [testing/probes-on-is-a-tested-configuration, testing/nvst-is-separate, config/the-extension-set-is-in-every-unit-key]
  - id: 5
    title: the conformance suite runs with the probes on
    status: done
    context: [crates/nvs-cli/src/main.rs, crates/nvs-runtime/src/ctx/trace.rs, .github/workflows/ci.yml, docs/rules/testing/probes-on-is-a-tested-configuration.md]
    extra:
      context:
        rules: [testing/probes-on-is-a-tested-configuration, testing/debug-probes, testing/nvst-is-separate]
  - id: 6
    title: the request-path gaps `request-overhead` handed over are built
    status: done
    context: [crates/nvs-host/src/cpuclock.rs, crates/nvs-host/src/isolate.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/stack.rs, crates/nvs-stdlib/src/format.rs, crates/nvs-stdlib/src/math.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/test.rs]
    extra:
      context:
        rules: [programs/memory-priority, testing/member-perf-ledger]
  - id: 7
    title: the goal is verified once, and every failure is fixed
    status: done
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

**`--opt-level` reaches a case's processes as `NOVIS_OPT_LEVEL`.** A `.nvst` case runs each `nvs`
process as a child, and a case may start more of its own, so `nvs test --opt-level` sets the level
in its own process and passes it to every child as `NOVIS_OPT_LEVEL=none|speed`
(`nvs_test::Options::nvs_env`). An environment variable is inherited by every grandchild, and an
argument is not. The variable replaces `NOVIS_NO_OPTIMISE`, and the CI lane names the flag. The
level sits in the unit key's content half (`crates/nvs-cli/src/cache.rs`'s `unit_key`), not in
`env_hash`, because `env_hash` is also `Core\Program::id()`'s and an id does not change with the
level. So the cache stays on at both levels. Request path: nothing, the key is hashed once per unit
on a cold read. Memory: one artifact per level for a unit that was compiled at both.

**`--probes` turns on one bit, `DebugFlags::SITES`, that sends every probe site to its helper and
records nothing.** Turning the recording bits on would change what a case prints: `TRACE` files query
spans and warns of a missing `debug.trace` grant, and `COVERAGE` counts into a table `Core\Debug`
reads. The bug class the rule names is the compiled half, a call at every site, and compiled code
tests the whole flags word, so one bit no helper reads exercises all of it. It is process-wide
(`nvs_runtime::probe_every_site`, as `set_opt_level` is), so every context the host makes has it,
and it reaches a case's processes as `NOVIS_PROBES`. `set_debug_flags` keeps it. Request path: one
relaxed load per context made. Memory: nothing. The rule's last paragraph records the declined
alternative.

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
in that order. Stage 6 touches only `crates/nvs-host/` and `crates/nvs-stdlib/` files no other stage
names, so it runs as a side lane in any order.

**The eight perf gaps `request-overhead` handed over are built here, as stage 6.** Each is latency on
the request path, which ranks above simplicity, and each record already states what closes it. No goal
ahead on the chain is about the request path, so moving them would only defer them. A record whose
closing turns out to cost an invariant every contributor must remember is struck as a stated bound in
its module doc instead, with the reason.

**A cross-tree switch keeps its CPU clock system call, as a stated bound.** No cheaper clock
charges every switch to the request it ran for. Linux's vDSO serves only wall clocks. The
`perf_event` task clock reads from a mapped page, but Docker's default seccomp profile and the
Debian and Ubuntu `perf_event_paranoid` default refuse it, so the clock `[limits] cpu_time` is
enforced on would gain a second path that CI does not run. Security and request isolation rank above
latency. A wall clock between switches and skipping the reads around the core's wait both charge a
request for time it did not spend. `crates/nvs-host/src/cpuclock.rs:49` is the home.
**An input fixed outside a bench is ramped by a sibling that prints it, and the bench is what is
measured.** A `.scale.nvs` sibling with `// bench: prints` returns, at each size, the bench's `.nvsr`,
`.in`, `.env` or program after `// scaling: file` lines (`tools/nv/cmd/scaling.ts`'s `printBeside`).
The ramp then measures the bench's own copy at its own `iterations`. Why: one mechanism covers a request,
standard input, the environment and generated classes, and it reuses the printed-program split that
`compile` ladders already use (simplicity). A template language in the tool, or one input kind per
input, was declined: each is more tool for the same reach. `.env` is a new proof sibling read by
`spawnProof` and hashed into the proof's definition. Request path: nothing. Memory: nothing.

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
- [x] A counted run that collects under a ceiling prints the same line cold and warm, the test above compares one of each, and the gap record is deleted (`rule:testing/bench-counters`). `crates/nvs-runtime/src/ctx/wiring.rs:121`, `crates/nvs-cli/tests/check_command.rs:331`

### 2. `#[Bench]` and `nvs test --bench` report the counted rows

**Does:** a recognized attribute, and a flag that runs benches and reports their counts.

**Gate:** `test-surface` stage 1, as `## Standing decisions` names it.

**Split:** once the attribute and flag land, one writing subagent writes `about.md`, the three examples,
the Novis test, the attack and the bench at the paths `bun nv proofs --id` prints, under § *Text an end
user reads*. The session keeps the Rust tests and the rule flip.

- [x] The rule settles the attribute's options and what `--bench` runs, as the standing decision says (`rule:testing/bench-counters`). `docs/rules/testing/bench-counters.md:17`
- [x] `#[Bench]` joins the recognized roster with its card, and its methods are collected beside `#[Test]`'s (`rule:testing/test-attribute`). `crates/nvs-types/src/derive.rs:73`, `crates/nvs-types/src/testing.rs:231`
- [x] `--bench` runs each bench under the counting mode and reports statements, calls, allocations, bytes and GC cycles, with wall time labelled advisory, in the text report and the JSON report, with its help text (`rule:testing/report-formats`). `crates/nvs-cli/src/runner.rs:142`, `crates/nvs-cli/src/runner.rs:2830`
- [x] A Rust test that a bench's rows are identical across two runs (`rule:testing/bench-counters`). `crates/nvs-cli/src/runner.rs:3458`
- [x] A heading for the feature in the testing chapter, and its feature proofs, with `bun nv proofs --id` on its id passing (`rule:testing/feature-proofs`). `docs/examples/lang/testing/chapter.md:15`, `docs/examples/lang/testing/bench-and-nvs-test-bench/about.md:1`
- [x] The rule `shipped`, its `**Not on disk.**` paragraph gone. `data/rules/testing/bench-counters.json:3`, `docs/rules/testing/bench-counters.md:17`

### 3. a bench ramps an input that is fixed outside its program

**Does:** a sibling form whose size sets a request body, standard input, the environment or a set of
generated classes.

**Split:** once the sibling form lands, writing subagents, one after another, write the siblings in the
four batches below. Each removes its benches' `// proof: gap nv/a-bench-cannot-ramp-an-input-fixed-outside-its-program`
line. The session checks each batch and ticks it.

- [x] A sibling form whose `N` sizes the input the program reads, such as a `.nvsr` or `.in` the ramp writes at each size (`rule:testing/member-perf-ledger`). `tools/nv/cmd/scaling.ts:919`
  - [x] `// bench: prints` in a `.scale.nvs` sibling prints the bench's input files, and the ramp measures the bench. `tools/nv/cmd/scaling.ts:580`, `tools/nv/cmd/scaling.ts:1067`
  - [x] A `<name>.env` sibling sets a proof's variables. `tools/nv/proofs/run.ts:311`
  - [x] The form is in the bench format's README, and `Request/body` is ramped by it. `benches/members/README.md:118`, `benches/members/core/Request/body.scale.nvs:1`
- [x] A test of the new form beside the existing ones (`rule:testing/perf-two-mechanisms`). `tools/nv/test/scaling.test.ts:58`
- [x] Batch 1, a request body: `Request/body`, `bodyStream`, `bytes`, `files`, `headers`, `json`, `jsonAs`, `postAs`, `queryAs`. `benches/members/core/Request/body.nvs:1`
- [x] Batch 2, a request part and a signed route: `Request-Part/content`, `readAll`, `saveTo`, `Router/signedRoute`. `benches/members/core/Request-Part/content.nvs:1`
- [x] Batch 3, standard input, the environment and the command line: `IO/stdin`, `Env/all`, `Command/completions`, `help`, `run`. `benches/members/core/IO/stdin.nvs:1`
- [x] Batch 4, generated classes: the two `lang/attributes/core-program-implementing*` benches. `benches/members/lang/attributes/core-program-implementing-i-every-class-that-is-an-i.nvs:1`
- [x] No bench carries the marker, and the gap record is deleted. `data/gaps/nv/a-bench-cannot-ramp-an-input-fixed-outside-its-program.json:1`

### 4. the conformance suite runs at each optimisation level

**Does:** `nvs test --opt-level`, and the CI lane uses it.

- [x] The rule names `--opt-level`, as the standing decision says (`rule:testing/probes-on-is-a-tested-configuration`). `docs/rules/testing/probes-on-is-a-tested-configuration.md:18`
- [x] Cranelift's level is a parameter of the host ISA in place of `NOVIS_NO_OPTIMISE`, and the compile cache keys on it (`rule:config/the-extension-set-is-in-every-unit-key`). `crates/nvs-codegen/src/lib.rs:896`, `crates/nvs-codegen/src/lib.rs:2076`, `crates/nvs-cli/src/cache.rs:1291`
- [x] `--opt-level` on `nvs test`, with its help text. `crates/nvs-cli/src/runner.rs:142`
- [x] A Rust test that a unit cached at one level is not loaded at the other (`rule:config/the-extension-set-is-in-every-unit-key`). `crates/nvs-cli/src/cache.rs:1585`
- [x] The CI lane runs `--opt-level none` in place of the variable; proved locally on `tests/conformance/lang/`. `.github/workflows/ci.yml:229`

### 5. the conformance suite runs with the probes on

**Does:** `nvs test --probes`, and a CI lane that uses it.

**Gate:** `trace-timeline` stage 6, as `## Standing decisions` names it.

- [x] The rule names `--probes` (`rule:testing/probes-on-is-a-tested-configuration`). `docs/rules/testing/probes-on-is-a-tested-configuration.md:21`
- [x] Over a `.nvst` tree, `--probes` runs each case with every probe site on and nothing recorded, and output is compared as usual (`rule:testing/debug-probes`, `## Standing decisions`). `crates/nvs-runtime/src/ctx/trace.rs:282`, `crates/nvs-cli/src/main.rs:2833`
- [x] A Rust test that a case passes with and without `--probes` and writes nothing more, and one that the sites record nothing (`rule:testing/nvst-is-separate`). `crates/nvs-cli/tests/nvst_select.rs:146`, `crates/nvs-runtime/src/ctx/trace.rs:1495`
- [x] CI runs the conformance suite once with the probes on, beside the existing run; proved locally on `tests/conformance/lang/`. `.github/workflows/ci.yml:303`
- [x] The rule `shipped` and its `**Not on disk.**` paragraph gone. `bun nv owners --closes bench-counters` names no designed rule; the gaps it names are stages 3's and 6's. `data/rules/testing/probes-on-is-a-tested-configuration.json:3`

### 6. the request-path gaps `request-overhead` handed over are built

**Does:** each of the eight gap records is built as its own text says, and its record is deleted.

**Split:** the items touch disjoint files, so writing subagents may take them side by side, and none
of them builds. The session builds once they all end, and checks each against its record.

- [x] The scheduler's task maps hash `TaskId` with `rustc_hash`, with a comment saying why the key needs no seed. `crates/nvs-host/src/scheduler.rs:382`, `data/gaps/nvs-host/a-task-id-is-hashed-with-siphash.json:1`
- [x] `Core\Math::intDiv` is emitted inline, with the same division-by-zero and overflow errors, and its bench declares the calls it makes. `crates/nvs-stdlib/src/math.rs:163`, `data/gaps/nvs-stdlib/math-int-div-is-an-out-of-line-call.json:1`
- [x] `Core\Test\Response::body` keeps the checked string after the first call, per response, and its bench declares what it measures. `crates/nvs-stdlib/src/test.rs:1`, `data/gaps/nvs-stdlib/a-test-response-body-copies-its-bytes-on-every-call.json:1`
- [x] `Core\Request::jsonAs` decodes with `read_against`, keeping every check `decodeAs` keeps, and every `Core\Request` and `Core\Json` attack gives the same output. `crates/nvs-stdlib/src/request.rs:336`, `data/gaps/nvs-stdlib/request-json-as-builds-arrays-before-its-object.json:1`
- [x] A literal `Str::format` template is split while compiling, a tainted argument's result stays tainted, and the bench declares what it measures. `crates/nvs-types/src/intrinsics.rs:905`, `crates/nvs-ir/src/lower/expr.rs:2437`
- [x] A cross-tree switch charges `[limits] cpu_time` with a cheaper clock, or the read is struck as a stated bound (`## Standing decisions`). `crates/nvs-host/src/cpuclock.rs:49`
- [x] The hello case's 2.5 µs of isolate start and finish is broken down, and each part that is not isolation is removed or moved out of the request. `crates/nvs-host/src/isolate.rs:1`, `data/gaps/nvs-host/an-isolate-start-and-finish-cost-microseconds-per-request.json:1`
- [x] A waiting request's stack and heap are broken down in the module doc, and each part not needed per request is removed. `crates/nvs-host/src/stack.rs:1`, `data/gaps/nvs-host/a-waiting-requests-memory-is-not-broken-down.json:1`
  - [x] A debug test measures a waiting request's heap on its core, connection and request apart. `crates/nvs-cli/src/serve.rs:4499`
  - [x] The heap is broken down in `stack.rs`'s *What a waiting request holds*. `crates/nvs-host/src/stack.rs:123`
  - [x] The per-connection read scratch and the head buffer the access line kept are gone, 16 KiB per waiting request. `crates/nvs-server/src/io.rs:508`, `crates/nvs-server/src/serve.rs:1682`
  - [x] The stack is counted per part through `nvs_host::counts::StackPages`, and the module doc breaks it down: the program's part is the host's path, and three quarters is the connection's. `crates/nvs-host/src/stack.rs:117`
  - [x] The task's `Ctx` stays boxed, one page less on every task's stack. `crates/nvs-host/src/scheduler.rs:923`
  - [x] The connection's stack, 8 pages while its request waits in release, is broken down by frame, and a frame not needed while it waits is made smaller or moved. `crates/nvs-server/src/serve.rs:1574`
  - [x] The gap record is deleted. `data/gaps/nvs-host/a-waiting-requests-memory-is-not-broken-down.json:1`

### 7. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [x] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [x] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`
- [x] Every stale perf figure the goal's perf commits left is recorded before the push, so CI's `proofs` job is green: `bun nv proofs --gate --limit 0` lists them (106 at `977773ab3`: `math.rs`, `test.rs`, `request.rs`, `json.rs`). `tools/nv/proofs/perf.ts:1`

## Handoff

### State

The goal is reached. Every stage is done, and every goal-end gate is green on `54cc4e88f`:
`bun nv verify --doc` (every link resolves), `bun nv owners --closes bench-counters` (owns no gap),
`bun nv proofs --gate --limit 0` (nothing owed in 1516 features) and `bun nv ci --check` (all ten
pre-push checks ok). Each standing decision is also in its home: `rule:testing/bench-counters`,
`rule:testing/probes-on-is-a-tested-configuration`, `benches/members/README.md`,
`crates/nvs-host/src/blocking.rs`, `crates/nvs-cli/src/cache.rs` and `crates/nvs-host/src/cpuclock.rs`.

- **The last six perf figures are recorded** (`54cc4e88f`). `Core\Request::body`, `postAs` and
  `queryAs` declare `calls 1`, `2` and `2`, and `bodyStream` declares `allocations 9`. Nine is per
  stream, not per chunk, and the declaration pins it, so no gap record was opened for it.
- **Callgrind measures each ramp size warm** (`e2d980359`, `tools/nv/cmd/scaling.ts:506`). One plain
  run fills the compile cache before the measured run. `Core\Test::assertTrue` and `assertSame` are
  now `unclear`, not a false `grows`: `bytes` still rises by 1 when the echoed total gains a digit.
- The sweep's reds were stale: `env-secret-proofs` exits 0, and nightly `40174915924` tested `9be30356e`,
  before the fixes on HEAD that are not pushed yet.

Tests expected to go red: none known.

### Next

**Retire the goal.** Nothing is left. The driver runs `owners-closes` and the floor on `DONE`, then retires the goal, and `after_retire` rebuilds `target/release/nvs`. `tools/nv/cmd/verify.ts:1`

### Backlog

- What a waiting connection still keeps is a stated bound in `stack.rs`: the parked drive's
  7.9 KiB frame, about 5.5 KiB of it temporaries no split moved, and the pages the request's start
  touched before it returned. Running the start on another stack would add a switch per request.
- `--opt-level` and `--probes` are new `nvs test` flags, and `bun nv proofs --gate` owes no proof for
  either.
- `bun nv scaling --budget` reports 45 benches over their share, none of them this goal's
  (`benches/members/core/Metrics/`, `lang/expressions/the-pipeline-operator`, `novis/Spreadsheet-*`).
  It is not a gate. Whoever owns those benches lowers N or writes the reason. `data/perf/bench-budget.json:1`
- The gate stage watches the nightly `writeStdin` perf proof on Linux for the leftover one-allocation
  difference seen above the ramp's max.
- A covws debug build counts more `allocations` than a release build (3133 against 2676 for one
  `Core\IO::write` program), while a plain `cargo build` counts 2676. The coverage
  instrumentation's allocations are in the count. The reference step runs the covws binary, so a
  reference example that prints `allocations=` will differ from the release figure.
- There are `.profraw` files under `crates/nvs-stdlib/src/` (git ignores them): a coverage run wrote
  them with that directory as its working directory.
