---
looper: 1
title: nvs test --mutate measures whether the tests would notice a wrong line
summary: "nvs test --mutate generates type-valid mutants from the checked program, never from test code, runs each against the tests that reach its line, reports what survived, has a bench of its cost and its feature proofs, and the checks below pass."
context: [crates/nvs-cli/src/runner.rs, crates/nvs-cli/src/coverage.rs, crates/nvs-ir/src/lower/mod.rs, docs/rules/testing/mutation-testing.md]
stages:
  - id: 1
    title: the mutant set
    status: done
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-types/src/testing.rs, docs/rules/testing/mutation-testing.md, docs/rules/testing/tests-never-reach-a-build.md]
    extra:
      context:
        rules: [testing/mutation-testing, testing/tests-never-reach-a-build, testing/debug-probes]
  - id: 2
    title: each mutant runs against the tests that reach its line, and the run reports what survived
    status: done
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
        rules: [testing/mutation-testing, testing/feature-proofs, testing/member-perf-ledger]
  - id: 4
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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

**How a mutant finds its tests, and what stops it (stage 2).** A mutant runs the tests that reached
the innermost statement whose span holds its changed expression, read off the coverage run's
statement numbers rather than its lines, so a change on a statement's second line is still reached.
A test with no wall-time limit gets ten times the slowest coverage-run test plus one second, because
otherwise a looping mutant hangs the run; one number for the run, not a per-test table (simplicity).
A coverage run with a failing test runs no mutant. One mutant runs at a time, its tests spread over
`--jobs`. `--format junit` is refused beside `--mutate`. A mutant whose run panics is killed. The
rule states each, with the declined alternatives.

**The bench is a `bun nv scaling` ladder, not a member bench (stage 3).** A member bench is a
`Bench::run` a Novis program calls, and a tool feature owes none on the roster, so the cost is a
`mutate` ladder kind: `benches/scaling/testing/mutate.nvs` prints a program and the tool runs `nvs
test --mutate --format json` on it, counting mutants and test runs (each JSON record's new `tests`
field). The ladder grows the suite around a fixed program under test, and `runsPerMutantGrow` fails
when runs per mutant grow with it. Why: growing the program too makes the clock quadratic by the
whole-program recompile already decided, which the rule now states; the ladder measures what the
covering-test filter decides. A proof program runs under `--mutate` by declaring `// proof: test
--mutate`.

**Lanes.** Stages run in order, 1 then 2 then 3: stage 2 consumes stage 1's mutant list from
`crates/nvs-ir/src/lower/mutate.rs`, and stage 3 measures stage 2's command and shares
`crates/nvs-cli/src/runner.rs` with it. Stage 1's rule item touches only
`docs/rules/testing/mutation-testing.md` and runs in any order, beside anything. Stage 1's code shares
`crates/nvs-ir/src/lower/mod.rs` with `test-surface` stage 1, and stage 2 shares
`crates/nvs-cli/src/runner.rs` with `test-surface` stage 5, so neither runs beside those.

## Stages

### 1. the mutant set

**Does:** a list of type-valid mutants for a program, each with the file, line and change it makes.

- [x] The rule states the six operators, the IR level, the whole-program recompile and the `.nvst` error, as the standing decisions settle them (`rule:testing/mutation-testing`). `docs/rules/testing/mutation-testing.md:13`
- [x] Gate: `test-surface` stage 1 `done`. Mutants are generated from the checked program at lowering, one changed site each, and every one lowers (`rule:testing/mutation-testing`). `crates/nvs-ir/src/lower/mutate.rs:1`, `crates/nvs-ir/src/lower/mod.rs:735`
- [x] Test code is never mutated, by the mark `test-surface` stage 1 adds (`rule:testing/tests-never-reach-a-build`). `crates/nvs-types/src/testing.rs:244`
- [x] Rust tests: each operator yields the expected mutants on a fixture, every mutant lowers, and a test method yields none (`rule:testing/mutation-testing`). `crates/nvs-ir/src/lower/mutate.rs:1`

### 2. each mutant runs against the tests that reach its line, and the run reports what survived

**Does:** `nvs test --mutate` runs the suite once with coverage, then each mutant against its covering
tests, and reports each mutant as killed, survived, timed out or not reached.

- [x] The rule states the four outcomes, the score, the JSON version and the exit status (`rule:testing/mutation-testing`). `docs/rules/testing/mutation-testing.md:13`
- [x] `--mutate` joins `nvs test`'s flags with its help text, and on a `.nvst` tree it exits with the coverage flags' error. `crates/nvs-cli/src/runner.rs:142`, `crates/nvs-cli/src/main.rs:2433`
- [x] Every mutant of a fixture compiles through `nvs_codegen::compile`, which stage 1's tests stop short of: they lower each one. `crates/nvs-cli/src/runner.rs:1168`, `crates/nvs-ir/src/lower/mutate.rs:1`
- [x] One coverage run maps each line to the tests that reached it (`rule:testing/coverage-report`). `crates/nvs-cli/src/runner.rs:1580`, `crates/nvs-cli/src/coverage.rs:243`
- [x] Each mutant runs only its covering tests, in parallel up to `--jobs`, and stops at its first failing test (`rule:testing/tests-run-across-every-core`). `crates/nvs-cli/src/runner.rs:1462`
- [x] Gate: `test-surface` stage 5 `done`. A mutant that loops is stopped by the test's time limit and counted as timed out (`rule:testing/a-test-has-a-wall-time-limit`). `crates/nvs-cli/src/runner.rs:1567`
- [x] The human report lists every survived mutant with file, line and change, and the JSON report's new version adds the `mutants` array (`rule:testing/report-formats`). `crates/nvs-cli/src/runner.rs:2830`
- [x] Rust tests: a fixture with one killed, one survived, one timed-out and one unreached mutant reports each, and the JSON form round-trips (`rule:testing/mutation-testing`). `crates/nvs-cli/src/runner.rs:3458`

### 3. a bench of its cost, and its feature proofs

**Does:** the cost of a mutation run is measured as it grows, and the feature owes nothing.

**Split:** the session adds the feature's heading and lands the bench; then writing subagents, one after
another: the three examples and `about.md`; then the attack and the Novis test. The session keeps the
Rust tests and the rule flip. Each subagent's brief names the paths `bun nv proofs --id
'tools:cli/nvs-test-mutate'` prints and § *Text an end user reads*.

- [ ] A `# nvs test --mutate` heading in the CLI chapter puts the feature on the roster. `docs/reference/tools/10-cli.md:302`
- [ ] A bench counts mutants and test runs on a fixture of growing size, and shows the covering-test filter keeps runs per mutant flat while the suite grows (`rule:testing/member-perf-ledger`). `benches/members/tools/cli/nvs-test-mutate.nvs:1`
- [ ] `about.md` and three examples (`rule:testing/feature-proofs`). `docs/examples/tools/cli/nvs-test-mutate/about.md:1`
- [ ] An attack where a mutant loops or allocates without end and is stopped, and a Novis test (`rule:testing/feature-proofs`). `tests/hostile/tools/cli/nvs-test-mutate/01-a-mutant-that-never-ends.nvs:1`
- [ ] `bun nv proofs --id 'tools:cli/nvs-test-mutate'` prints nothing owed. `tools/nv/proofs/roster.ts:209`
- [ ] The rule `shipped`, its `**Not on disk.**` paragraph gone, and `bun nv owners --closes mutation-testing` passes (`rule:testing/mutation-testing`). `data/rules/testing/mutation-testing.json:3`, `docs/rules/testing/mutation-testing.md:15`

### 4. the goal is verified once, and every failure is fixed

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

Stages 1 and 2 are done. `nvs test --mutate` runs the selected tests once with coverage, then each
mutant against the tests that reached it, and reports `killed`, `survived`, `timed out` or `not
reached` (`crates/nvs-cli/src/runner/mutation.rs`, wired in `runner::run`). A mutant's tests are
those that reached the innermost statement holding its span (`mutation::innermost`, over
`coverage::Probes::stmt_spans`). Each mutant compiles with `cache::unit_for(.., None)` and runs
through `run_suite_in_a_task`, which now takes `Rc<Checked>` so the run keeps the checked program.
A mutant's run is `Format::Json` into a buffer, `fail_fast: true`, and `Shape::wall`, the fallback
limit `wall_limit` applies to a test with none (10 × the slowest coverage-run test + 1 s). Timed out
is read off `PAST_WALL_TIME`, the start of `Outcome::past_wall_time`'s line. JSON is version 4 with a
`mutants` array; `--format junit` with `--mutate`, and `--mutate` on a `.nvst` tree, exit with an
error. The rule, `report-formats` and `docs/reference/tools/10-cli.md` (one bullet under `nvs
test`) say so. The rule's `**Not on disk.**` paragraph is gone; its status is still `designed`.

Checked: `cargo build`, `cargo clippy --all-targets -- -D warnings`, and the two new tests ran once
and pass (`runner::mutation::tests`). By hand, `nvs test crates/nvs-cli/tests/fixtures/runner/mutate.nvs
--mutate` reports 3 killed, 1 survived, 1 timed out, 1 not reached, score 80.0%.

The sweep's `env-secret-proofs` is green on disk (its log shows exit 0). The nightly's five red jobs
tested `3ea6f1550`, and each has a fix on main since `10c402b57`; the next nightly is their check.

Expected red in stage 4: none known. Every `Shape` literal gained `wall: None`, every
`json_document` call a fifth `None`, and `Flags` a `mutate` field (`crates/nvs-cli/src/ext.rs` too).

### Next

**Stage 3: a bench of its cost, and its feature proofs.** Follow the stage's `**Split:**` line. The
feature id is `tools:cli/nvs-test-mutate`; start with `bun nv proofs --id 'tools:cli/nvs-test-mutate'`
once the heading exists.

- [ ] A `# nvs test --mutate` heading in the CLI chapter; move the `--mutate` bullet under it. `docs/reference/tools/10-cli.md:276`
- [ ] The bench over a growing fixture: mutants and test runs per mutant. `benches/members/README.md:1`
- [ ] `about.md` and three examples, by a writing subagent. `docs/examples/tools/cli/nvs-test-mutate/about.md:1`
- [ ] The attack (a mutant that never ends is stopped by `Shape::wall`) and a Novis test. `crates/nvs-cli/src/runner/mutation.rs:253`
- [ ] The rule `shipped` and `bun nv owners --closes mutation-testing` passes. `data/rules/testing/mutation-testing.json:3`

### Backlog

- A threshold flag that fails the run below a mutation score (`docs/rules/testing/mutation-testing.md`).
- More operators, such as a removed call statement, added in `crates/nvs-ir/src/lower/mutate.rs`.
- From `test-surface`: a test past its `[limits] cpu_time` reports the runtime's line, which names a request. A line naming the test and its limit would match `Outcome::past_wall_time`. `crates/nvs-cli/src/runner.rs:477`
- From `test-surface`: a property's counterexample prints an enum case as its number, such as `new Item("", 1, 0)`. The case name would read better. `crates/nvs-cli/src/runner.rs:2357`
- From `test-surface`: a long failing string or list spends the 1000-call shrink budget on one-item removals, so its counterexample stays long. Removing chunks first would shrink further. `crates/nvs-types/src/property/shrink.rs:362`
- From `test-surface`: a workspace folder with programs and no `.nvst` case pays one compile of the folder when the editor lists its cases, and the client reads none of those records. `editors/vscode/src/tests.ts:143`
- From `test-surface`: a case label mixes separators on Windows, such as `tests/conformance/core\x.nvst`, in a report and a listing. `crates/nvs-test/src/lib.rs:225`
- From `test-surface`: the "it asserted nothing" report line names a rule id, and text a user reads names none. `crates/nvs-cli/src/runner.rs:3375`
- `bun nv scaling --budget` names 45 benches over their stored share, among them `core/Metrics/*`, `novis/Spreadsheet-*` and `the-pipeline-operator`, none of them a `Core\Test` bench. `data/perf/bench-budget.json:1`
