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
    context: [crates/nvs-cli/src/runner.rs, docs/rules/testing/mutation-testing.md, docs/rules/testing/feature-proofs.md, docs/reference/tools/10-cli.md, benches/members/README.md, tools/nv/proofs/roster.ts]
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
---

# nvs test --mutate measures whether the tests would notice a wrong line

## Why

Coverage says which lines ran under a test. It does not say whether any assertion would have failed
had a line been wrong, and a suite at full line coverage can still miss most bugs. A mutation run
measures exactly that: it changes one thing, runs the tests, and records whether any failed.
`rule:testing/mutation-testing` designs it inside the compiler for two reasons an external tool cannot
match. Mutants come from the checked program, so each one compiles. And each mutant runs only against
the tests whose per-test coverage reached its line, which the runner already records
(`crates/nvs-cli/src/runner.rs:442`, `crates/nvs-cli/src/coverage.rs:243`). Nothing generates or runs a
mutant today.

## Standing decisions

**The goal runs after `test-surface`, by two gates.** A mutant must never be made from test code, and
`test-surface`'s stage 1 is what marks test code: its gate is that stage `status: done` in
`docs/goals/test-surface.md` (or that file gone) and no `**Not on disk.**` paragraph left in
`docs/rules/testing/tests-never-reach-a-build.md`. A mutant can loop forever, and `test-surface`'s stage 5
gives each test the time limit that stops it: its gate is that stage `done` and no `**Not on disk.**`
paragraph in `docs/rules/testing/a-test-has-a-wall-time-limit.md`. Stage 1's code waits on the first,
stage 2 on the second. Stage 1's rule item waits on nothing.

**The first operator set is six type-preserving swaps:** a relational boundary (`<`↔`<=`, `>`↔`>=`),
equality negated (`==`↔`!=`), an arithmetic swap (`+`↔`-`, `*`↔`/`), a logical connector swap
(`&&`↔`||`), a boolean literal flipped, and a branch condition negated. Why: each keeps the operand
and result types, so every mutant compiles with no re-check, and the six are the cheap ones most
likely to reveal a missing assertion. More operators are added in the code that applies them, and no
rule changes (simplicity).

**A mutant is made on the typed IR at lowering, one changed site per mutant,** in a new
`crates/nvs-ir/src/lower/mutate.rs`, keyed by the site's file, line and change. Why: the IR has the
types the swaps need and the spans the report names; a syntax-tree mutant would pay a check each.

**A mutant recompiles the whole program from the checked tree:** parse and check run once, lowering
and codegen once per mutant. Why: no partial-recompile mechanism to build or keep correct
(simplicity); the cost is one compile per mutant, which the trade-offs below already state.

**`--mutate` takes a program's `#[Test]` methods only.** On a `.nvst` tree it exits with an error, as
the coverage flags do (`crates/nvs-cli/src/main.rs:2433`). Why: per-test coverage exists only for program
tests.

**The report.** Each mutant is `killed`, `survived`, `timed out` or `not reached`. A mutant no test
reaches is `not reached`, runs nothing, and is left out of the score. The score is killed plus timed out
over the reached mutants. The human report lists every survived mutant with file, line and change. The
JSON report under `rule:testing/report-formats` is a new version that adds a `mutants` array, one record
per mutant with those four fields. The exit status fails the run only when a test fails without any
mutant. There is no threshold flag in this goal; it is a backlog item.

**Trade-offs.** Request path: nothing, and `nvs test` without `--mutate` pays nothing. A mutation run
costs one coverage run, then one compile and the covering tests for each mutant. Memory is O(`--jobs`),
the mutants in flight. Usability: one flag on `nvs test` gives a measurement teams otherwise set up
with a separate tool. Simplicity: the operator set lives in the code that applies it, so adding one
changes no rule.

**Lanes.** Stages run in order, 1 then 2 then 3: stage 2 consumes stage 1's mutant list from
`crates/nvs-ir/src/lower/mutate.rs`, and stage 3 measures stage 2's command and shares
`crates/nvs-cli/src/runner.rs` with it. Stage 1's rule item touches only
`docs/rules/testing/mutation-testing.md` and runs in any order, beside anything. Stage 1's code shares
`crates/nvs-ir/src/lower/mod.rs` with `test-surface` stage 1, and stage 2 shares
`crates/nvs-cli/src/runner.rs` with `test-surface` stage 5, so neither runs beside those.

## Stages

### 1. the mutant set

**Does:** a list of type-valid mutants for a program, each with the file, line and change it makes.

- [ ] The rule states the six operators, the IR level, the whole-program recompile and the `.nvst` error, as the standing decisions settle them (`rule:testing/mutation-testing`). `docs/rules/testing/mutation-testing.md:13`
- [ ] Gate: `test-surface` stage 1 `done`. Mutants are generated from the checked program at lowering, one changed site each, and every one compiles (`rule:testing/mutation-testing`). `crates/nvs-ir/src/lower/mutate.rs:1`, `crates/nvs-ir/src/lower/mod.rs:735`
- [ ] Test code is never mutated, by the mark `test-surface` stage 1 adds (`rule:testing/tests-never-reach-a-build`). `crates/nvs-types/src/testing.rs:244`
- [ ] Rust tests: each operator yields the expected mutants on a fixture, every mutant lowers, and a test method yields none (`rule:testing/mutation-testing`). `crates/nvs-ir/src/lower/mutate.rs:1`

### 2. each mutant runs against the tests that reach its line, and the run reports what survived

**Does:** `nvs test --mutate` runs the suite once with coverage, then each mutant against its covering
tests, and reports each mutant as killed, survived, timed out or not reached.

- [ ] The rule states the four outcomes, the score, the JSON version and the exit status (`rule:testing/mutation-testing`). `docs/rules/testing/mutation-testing.md:13`
- [ ] `--mutate` joins `nvs test`'s flags with its help text, and on a `.nvst` tree it exits with the coverage flags' error. `crates/nvs-cli/src/runner.rs:142`, `crates/nvs-cli/src/main.rs:2433`
- [ ] One coverage run maps each line to the tests that reached it (`rule:testing/coverage-report`). `crates/nvs-cli/src/runner.rs:1580`, `crates/nvs-cli/src/coverage.rs:243`
- [ ] Each mutant runs only its covering tests, in parallel up to `--jobs`, and stops at its first failing test (`rule:testing/tests-run-across-every-core`). `crates/nvs-cli/src/runner.rs:1462`
- [ ] Gate: `test-surface` stage 5 `done`. A mutant that loops is stopped by the test's time limit and counted as timed out (`rule:testing/a-test-has-a-wall-time-limit`). `crates/nvs-cli/src/runner.rs:1567`
- [ ] The human report lists every survived mutant with file, line and change, and the JSON report's new version adds the `mutants` array (`rule:testing/report-formats`). `crates/nvs-cli/src/runner.rs:2830`
- [ ] Rust tests: a fixture with one killed, one survived, one timed-out and one unreached mutant reports each, and the JSON form round-trips (`rule:testing/mutation-testing`). `crates/nvs-cli/src/runner.rs:3458`

### 3. a bench of its cost, and its feature proofs

**Does:** the cost of a mutation run is measured as it grows, and the feature owes nothing.

**Split:** the session adds the feature's heading and lands the bench; then writing subagents, one after
another: the three examples and `about.md`; then the attack and the Novis test. The session keeps the
Rust tests and the rule flip. Each subagent's brief names the paths `bun nv proofs --id
'tools:cli/nvs-test-mutate'` prints and § *Text an end user reads*.

- [ ] A `# nvs test --mutate` heading in the CLI chapter puts the feature on the roster. `docs/reference/tools/10-cli.md:302`
- [ ] A bench counts mutants and test runs on a fixture of growing size, and shows the covering-test filter keeps runs per mutant flat while the suite grows (`rule:testing/userland-benchmarks`). `benches/members/tools/cli/nvs-test-mutate.nvs:1`
- [ ] `about.md` and three examples (`rule:testing/feature-proofs`). `docs/examples/tools/cli/nvs-test-mutate/about.md:1`
- [ ] An attack where a mutant loops or allocates without end and is stopped, and a Novis test (`rule:testing/feature-proofs`). `tests/hostile/tools/cli/nvs-test-mutate/01-a-mutant-that-never-ends.nvs:1`
- [ ] `bun nv proofs --id 'tools:cli/nvs-test-mutate'` prints nothing owed. `tools/nv/proofs/roster.ts:209`
- [ ] The rule `shipped`, its `**Not on disk.**` paragraph gone, and `bun nv owners --closes mutation-testing` passes (`rule:testing/mutation-testing`). `data/rules/testing/mutation-testing.json:3`, `docs/rules/testing/mutation-testing.md:15`

## Handoff

### State

No stage is started. Stage 1's rule item needs nothing. Stage 1's code waits on `test-surface` stage 1
(`docs/goals/test-surface.md` shows it `done`, and `docs/rules/testing/tests-never-reach-a-build.md` has
no `**Not on disk.**` paragraph). Stage 2's time-limit item waits on `test-surface` stage 5 the same way
(`docs/rules/testing/a-test-has-a-wall-time-limit.md`).

### Next

**Stage 1: the mutant set**, file set `docs/rules/testing/mutation-testing.md`, then
`crates/nvs-ir/src/lower/`. The rule item has no gate; the three code items wait on `test-surface`
stage 1. If that gate does not hold, land the rule item alone and end the group.

- [ ] State the six operators, the IR level, the whole-program recompile and the `.nvst` error (`rule:testing/mutation-testing`). `docs/rules/testing/mutation-testing.md:13`
- [ ] Generate one-site mutants at lowering (`rule:testing/mutation-testing`). `crates/nvs-ir/src/lower/mutate.rs:1`, `crates/nvs-ir/src/lower/mod.rs:735`
- [ ] Skip test code by `test-surface`'s mark (`rule:testing/tests-never-reach-a-build`). `crates/nvs-types/src/testing.rs:244`
- [ ] Rust tests for each operator, every mutant lowering, and no mutant from a test method. `crates/nvs-ir/src/lower/mutate.rs:1`

### Backlog

- A threshold flag that fails the run below a mutation score (`docs/rules/testing/mutation-testing.md`).
- More operators, such as a removed call statement, added in `crates/nvs-ir/src/lower/mutate.rs`.
