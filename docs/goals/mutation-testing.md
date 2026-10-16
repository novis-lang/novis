---
looper: 1
title: nvs test --mutate measures whether the tests would notice a wrong line
summary: "nvs test --mutate generates type-valid mutants from the checked program, never from test code, runs each against the tests that reach its line, reports what survived, has a bench of its cost and its feature proofs, and the checks below pass."
context: [crates/nvs-cli/src/runner.rs, crates/nvs-cli/src/coverage.rs, crates/nvs-ir/src/lower/mod.rs, docs/rules/testing/mutation-testing.md]
stages:
  - id: 1
    title: the mutant set
    status: open
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-types/src/testing.rs, docs/rules/testing/mutation-testing.md, docs/rules/testing/tests-never-reach-a-build.md]
    extra:
      context:
        rules: [testing/mutation-testing, testing/tests-never-reach-a-build, testing/debug-probes]
  - id: 2
    title: each mutant runs against the tests that reach its line, and the run reports what survived
    status: open
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-cli/src/coverage.rs, crates/nvs-cli/src/main.rs, docs/rules/testing/mutation-testing.md, docs/rules/testing/a-test-has-a-wall-time-limit.md]
    extra:
      context:
        rules: [testing/mutation-testing, testing/coverage-report, testing/report-formats, testing/a-test-has-a-wall-time-limit, testing/tests-run-across-every-core]
  - id: 3
    title: a bench of its cost, and its feature proofs
    status: open
    context: [crates/nvs-cli/src/runner.rs, docs/rules/testing/mutation-testing.md, docs/rules/testing/feature-proofs.md]
    extra:
      context:
        rules: [testing/mutation-testing, testing/feature-proofs, testing/userland-benchmarks]
checks:
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, mutation-testing]
    reach: true
    floor: false
extra:
  milestone: M10
---

# nvs test --mutate measures whether the tests would notice a wrong line

## Why

Coverage says which lines ran under a test. It does not say whether any assertion would have failed
had a line been wrong, and a suite at full line coverage can still miss most bugs. A mutation run
measures exactly that: it changes one thing, runs the tests, and records whether any failed.
`rule:testing/mutation-testing` designs it inside the compiler for two reasons an external tool cannot
match. Mutants come from the checked program, so each one compiles. And each mutant runs only against
the tests whose per-test coverage reached its line, which the runner already records
(`crates/nvs-cli/src/runner.rs:2642-2647`). Nothing generates or runs a mutant today.

## Standing decisions

**The goal runs after `test-surface`.** The planning session's call. A mutant must never be made from
test code, and `test-surface`'s stage 1 is what marks test code. A mutant can loop forever, and
`test-surface`'s stage 5 gives each test the time limit that stops it.

**Open points are settled in the rule before the code.** The planning session's call. Stage 1 settles
the first operator set, the level a mutant is made at, and whether a mutant recompiles the whole
program or only the function it changes. Stage 2 settles the report's shape, what a mutant that no test
reaches counts as, and the exit status. The safe default for the exit status is a report that fails the
run only when a test fails without any mutant. A threshold that fails the run is a flag the rule may add.
A point with no safe default goes to the user.

**Trade-offs.** Request path: nothing, and `nvs test` without `--mutate` pays nothing. A mutation run
costs one coverage run, then one compile and the covering tests for each mutant. Memory is O(`--jobs`),
the mutants in flight. Usability: one flag on `nvs test` gives a measurement teams otherwise set up
with a separate tool. Simplicity: the operator set lives in the code that applies it, so adding one
changes no rule.

## Stages

### 1. the mutant set

**Does:** a list of type-valid mutants for a program, each with the file, line and change it makes.

- [ ] The rule settles the first operator set, the level a mutant is made at, and what a mutant recompiles (`rule:testing/mutation-testing`). `docs/rules/testing/mutation-testing.md`
- [ ] Mutants are generated from the checked program at lowering, one changed site each, and every one compiles. `crates/nvs-ir/src/lower/mod.rs:733`
- [ ] Test code is never mutated, by the mark `test-surface` adds (`rule:testing/tests-never-reach-a-build`). `crates/nvs-types/src/testing.rs:241`
- [ ] Tests that each operator yields the expected mutants on a fixture, and that a test method yields none.

### 2. each mutant runs against the tests that reach its line, and the run reports what survived

**Does:** `nvs test --mutate` runs the suite once with coverage, then each mutant against its covering
tests, and reports each mutant as killed, survived, timed out or not reached.

- [ ] The rule settles the report's shape, the counts, and the exit status. `docs/rules/testing/mutation-testing.md`
- [ ] One coverage run maps each line to the tests that reached it. `crates/nvs-cli/src/runner.rs:2642-2647`, `crates/nvs-cli/src/coverage.rs:243`
- [ ] Each mutant runs only its covering tests, in parallel up to `--jobs`, and stops at its first failing test (`rule:testing/tests-run-across-every-core`).
- [ ] A mutant that loops is stopped by the test's time limit and counted as timed out (`rule:testing/a-test-has-a-wall-time-limit`).
- [ ] A human report that lists every survived mutant with its file, line and change, and a JSON form under `rule:testing/report-formats`. The help text in the binary.

### 3. a bench of its cost, and its feature proofs

**Does:** the cost of a mutation run is measured as it grows, and the feature owes nothing.

- [ ] A bench counts mutants and test runs on a fixture of growing size, and shows the covering-test filter keeps runs per mutant flat while the suite grows.
- [ ] The feature proofs: `about.md`, tests from Novis and from Rust, three examples, the bench, an attack where a mutant loops or allocates without end and is stopped, and the help (`rule:testing/feature-proofs`). `bun nv proofs --id` once the roster names it.
- [ ] The rule `shipped`, and `bun nv owners --closes mutation-testing` passes.

## Handoff

### State

The goal is new. No stage is started. It waits for `test-surface`'s stages 1 and 5. The anchors above
were checked against the tree when it was written.

### Next

Stage 1, the mutant set, on `crates/nvs-ir/src/lower/mod.rs` and `docs/rules/testing/mutation-testing.md`:
- settle the operators, the level and the recompile in `rule:testing/mutation-testing`.
- generate one-site mutants at lowering. `crates/nvs-ir/src/lower/mod.rs:733`
- skip test code by `test-surface`'s mark. `crates/nvs-types/src/testing.rs:241`

### Backlog

- A threshold flag that fails the run below a mutation score, if stage 2's rule adds one (`docs/rules/testing/mutation-testing.md`).
