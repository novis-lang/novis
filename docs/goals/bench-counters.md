---
looper: 1
title: "Benches report counted work that CI can gate on, and the probes and optimisation levels are tested configurations"
summary: "#[Bench] and nvs test --bench report statements, calls, allocations, bytes and GC cycles with wall time advisory, a blocking call counts the same on every run, a bench ramps an input fixed outside its program, CI runs the conformance suite with the probes on and once per optimisation level, every designed rule and gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs]
stages:
  - id: 1
    title: a GC cycle is counted, and a blocking call counts the same on every run
    status: open
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-runtime/src/object.rs, crates/nvs-cli/src/main.rs, crates/nvs-host/src/blocking.rs, tools/nv/proofs/perf.ts, data/gaps/nvs-host/a-blocking-call-allocates-by-when-it-ends.json]
    extra:
      context:
        rules: [testing/bench-counters, testing/member-perf-ledger, observability/gc-pause-is-its-own-event]
  - id: 2
    title: "`#[Bench]` and `nvs test --bench` report the counted rows"
    status: open
    context: [crates/nvs-types/src/derive.rs, crates/nvs-types/src/testing.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/bench-counters.md]
    extra:
      context:
        rules: [testing/bench-counters, testing/test-attribute, testing/report-formats, testing/tests-never-reach-a-build]
  - id: 3
    title: a bench ramps an input that is fixed outside its program
    status: open
    context: [tools/nv/cmd/scaling.ts, tools/nv/test/scaling.test.ts, data/gaps/nv/a-bench-cannot-ramp-an-input-fixed-outside-its-program.json]
    extra:
      context:
        rules: [testing/member-perf-ledger, testing/perf-two-mechanisms]
  - id: 4
    title: the conformance suite runs with the probes on and at each optimisation level
    status: open
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/cache.rs, .github/workflows/ci.yml, docs/rules/testing/probes-on-is-a-tested-configuration.md]
    extra:
      context:
        rules: [testing/probes-on-is-a-tested-configuration, testing/debug-probes, testing/nvst-is-separate, config/the-extension-set-is-in-every-unit-key]
checks:
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, bench-counters]
    reach: true
    floor: false
extra:
  milestone: M10
---

# Benches report counted work that CI can gate on, and the probes and optimisation levels are tested configurations

## Why

`nvs run --count` prints four counts that are the same on every machine (`crates/nvs-cli/src/main.rs:439-451`),
and a program's author cannot use them: `#[Bench]` is not a recognized attribute
(`crates/nvs-types/src/derive.rs:73`), so a CI pipeline cannot gate on its own code's work. The counts
lack garbage-collection cycles (`crates/nvs-runtime/src/ctx/trace.rs:578-583`). Two gap records make
this repository's own benches noisy: a call on the blocking pool allocates a different amount from run
to run, and a bench cannot ramp an input that is fixed before its program starts. And the probes and
Cranelift's other optimisation levels are shapes that no test run takes
(`rule:testing/probes-on-is-a-tested-configuration`), which is where a JIT's instrumentation bugs live.

## Standing decisions

**A bench is test code that runs only under `--bench`.** The planning session's call. `#[Bench]`
marks a method in a class that `nvs test` discovers. A plain `nvs test` skips it, and `nvs test --bench`
runs the benches and no tests. It is test code under `rule:testing/tests-never-reach-a-build`, so
`test-surface`'s stage 1 keeps it out of builds. Stage 2 writes the attribute's options into
`rule:testing/bench-counters` before the code.

**The GC count joins the count line at its end.** The planning session's call. `--count` prints
`gc=<cycles>` after `bytes`, so `tools/nv/proofs/perf.ts:49`, which matches from the start of the line,
keeps reading the four it reads today.

**Two `nvs test` flags select the tested configurations.** The user decided this. A flag that
turns every probe bit on for each case, and one that picks Cranelift's optimisation level, both named in
`rule:testing/probes-on-is-a-tested-configuration` before the code. They are flags because the CI lane
runs the built binary, and a flag is visible in `nvs test --help` where an environment variable is not.
The compile cache keys on the level, so a unit compiled at one level is never run at another.

**Trade-offs, all together.** Request path: nothing. A counted run does one more increment per
collection. Memory: one integer per run for the GC count, and nothing per bench beyond its row in the
report. Usability: a bench that gates in CI on every operating system, with wall time printed and
labelled advisory. Simplicity: one attribute and three flags. CI time grows by one conformance run per
added configuration.

## Stages

### 1. a GC cycle is counted, and a blocking call counts the same on every run

**Does:** the counts gain GC cycles, and a call run on the blocking pool allocates the same amount
however its job ends.

- [ ] `Counted` gains a GC cycle count, raised where a collection runs, and `--count` prints it last on its line (`rule:testing/bench-counters`). `crates/nvs-runtime/src/ctx/trace.rs:578-583`, `crates/nvs-runtime/src/object.rs:3011`, `crates/nvs-cli/src/main.rs:3746-3754`
- [ ] The blocking pool's park and wake allocate once per task, not once per call, so their allocations do not depend on when the answer lands. `crates/nvs-host/src/blocking.rs:348-362`
- [ ] The `writeStdin` bench drops its gap marker and judges the same on repeated runs, and `data/gaps/nvs-host/a-blocking-call-allocates-by-when-it-ends.json` is deleted. `benches/members/core/Process-Handle/writeStdin.nvs:6`
- [ ] A test that two counted runs of a program with a collection print the same line.

### 2. `#[Bench]` and `nvs test --bench` report the counted rows

**Does:** a recognized attribute, and a flag that runs benches and reports their counts.

- [ ] The rule settles the attribute's options and what `--bench` runs, as the standing decision says (`rule:testing/bench-counters`). `docs/rules/testing/bench-counters.md`
- [ ] `#[Bench]` joins the recognized roster with its card, and its methods are collected beside `#[Test]`'s. `crates/nvs-types/src/derive.rs:73`, `crates/nvs-types/src/testing.rs:231-239`
- [ ] `--bench` runs each bench under the counting mode and reports statements, calls, allocations, bytes and GC cycles, with wall time labelled advisory, in the text and JSON reports. `crates/nvs-cli/src/main.rs:574`, `crates/nvs-cli/src/runner.rs:2647`
- [ ] Tests that a bench's rows are identical across two runs, examples, the help text, and `bun nv proofs --id` on the attribute's id passes. The rule `shipped`.

### 3. a bench ramps an input that is fixed outside its program

**Does:** a sibling form whose size sets a request body, standard input, the environment or a set of
generated classes.

- [ ] A sibling form whose `N` sizes the input the program reads, such as a `.nvsr` or `.in` the ramp writes at each size. `tools/nv/cmd/scaling.ts:919-922`
- [ ] One sibling for each bench that carries the gap's marker, and every marker removed.
- [ ] A test of the new form beside the existing ones, and `data/gaps/nv/a-bench-cannot-ramp-an-input-fixed-outside-its-program.json` deleted. `tools/nv/test/scaling.test.ts`

### 4. the conformance suite runs with the probes on and at each optimisation level

**Does:** two flags on `nvs test`, and CI lanes that use them.

- [ ] The rule names the two flags, as the standing decision says (`rule:testing/probes-on-is-a-tested-configuration`). `docs/rules/testing/probes-on-is-a-tested-configuration.md`
- [ ] Cranelift's level is a parameter of the host ISA, and the compile cache keys on it. `crates/nvs-codegen/src/lib.rs:1965-1973`, `crates/nvs-cli/src/cache.rs:224`
- [ ] Over a `.nvst` tree, the probe flag runs each case with every probe bit on and its data discarded, and output is compared as usual. `crates/nvs-cli/src/main.rs:3912`
- [ ] CI runs the conformance suite once with the probes on and once per further level, beside the existing run. `.github/workflows/ci.yml:220-221`
- [ ] The rule `shipped`, and `bun nv owners --closes bench-counters` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, the GC count and the blocking pool, on `crates/nvs-runtime/src/ctx/trace.rs` and
`crates/nvs-host/src/blocking.rs`:
- the GC cycle count in `Counted` and on the count line (`rule:testing/bench-counters`). `crates/nvs-runtime/src/ctx/trace.rs:578`
- one park and wake allocation per task. `crates/nvs-host/src/blocking.rs:348`

### Backlog

- Stage 2 needs `test-surface`'s stage 1, so a bench never reaches a build (`docs/rules/testing/tests-never-reach-a-build.md`).
- Stage 4's trace bit writes to the sink `trace-timeline` builds (`docs/rules/testing/debug-surface.md`).
