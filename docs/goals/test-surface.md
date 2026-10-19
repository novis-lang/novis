---
looper: 1
title: Tests stay out of builds, and the test runner has the features a team expects
summary: "No test reaches nvs run or nvs build, a test reaches private members in its own file, #[Property] runs, nvs test --list covers .nvst trees, the time limit, tags, shards, fail-fast, rerun-failed, shuffle and watch work, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/runner.rs, crates/nvs-types/src/testing.rs]
stages:
  - id: 1
    title: tests are absent from `nvs run` and `nvs build`
    status: open
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/script.rs, crates/nvs-types/src/testing.rs, crates/nvs-types/src/check.rs, crates/nvs-runtime/src/ctx/error.rs, docs/rules/testing/tests-never-reach-a-build.md]
    extra:
      context:
        rules: [testing/tests-never-reach-a-build, testing/test-attribute, testing/fixtures, testing/class-hooks]
  - id: 5
    title: a test has a time limit, and tags and shards select what runs
    status: open
    context: [crates/nvs-types/src/testing.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/a-test-has-a-wall-time-limit.md, docs/rules/testing/tags-and-shards-select-what-runs.md]
    extra:
      context:
        rules: [testing/a-test-has-a-wall-time-limit, testing/tags-and-shards-select-what-runs, testing/test-attribute, errors/on-limit]
  - id: 2
    title: a test reaches `private` members of classes in its own file
    status: open
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/testing.rs, docs/rules/testing/private-in-the-same-file.md]
    extra:
      context:
        rules: [testing/private-in-the-same-file, testing/tests-never-reach-a-build]
  - id: 3
    title: "`#[Property]` is recognized, and its inputs are generated from the declared types"
    status: open
    context: [crates/nvs-types/src/derive.rs, crates/nvs-types/src/testing.rs, docs/rules/testing/property-testing.md]
    extra:
      context:
        rules: [testing/property-testing, testing/determinism-declared-on-the-test]
  - id: 7
    title: "a `#[Property]` failure shrinks, and the report carries the counterexample and the seed"
    status: open
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-types/src/testing.rs, docs/rules/testing/property-testing.md]
    extra:
      context:
        rules: [testing/property-testing, testing/data-rows, testing/report-formats, testing/determinism-declared-on-the-test]
  - id: 8
    title: "`#[Property]` has its feature proofs"
    status: open
    context: [docs/examples/lang/testing/chapter.md, docs/rules/testing/property-testing.md, docs/rules/testing/feature-proofs.md]
    extra:
      context:
        rules: [testing/property-testing, testing/feature-proofs]
  - id: 4
    title: "`nvs test --list` covers `.nvst` trees, and the editor stops searching for them"
    status: open
    context: [crates/nvs-cli/src/runner.rs, editors/vscode/src/tests.ts, editors/vscode/test/surfaces/tests.test.ts, docs/rules/ide/the-test-tree-is-discovered-and-run-through-the-cli.md]
    extra:
      context:
        rules: [ide/the-test-tree-is-discovered-and-run-through-the-cli, testing/report-formats, testing/nvst-is-separate]
  - id: 6
    title: fail-fast, rerun-failed and shuffle
    status: open
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-config/src/data.rs, docs/rules/testing/a-run-can-stop-early-rerun-failures-and-shuffle.md]
    extra:
      context:
        rules: [testing/a-run-can-stop-early-rerun-failures-and-shuffle, testing/runner-is-strict]
  - id: 9
    title: "`nvs test --watch`"
    status: open
    context: [crates/nvs-cli/src/script.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/watch-reruns-on-an-edit.md]
    extra:
      context:
        rules: [testing/watch-reruns-on-an-edit, config/an-edit-reaches-the-next-request-without-a-restart]
checks:
  - id: owners-closes
    stage: 9
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, test-surface]
    reach: true
    floor: false
---

# Tests stay out of builds, and the test runner has the features a team expects

## Why

Three designed testing rules are not on disk. `nvs run` and `nvs build` lower every `#[Test]` method
into the artifact (`crates/nvs-ir/src/lower/mod.rs:776`), so production carries the test code its rule
says it never sees. A test in the same file as a class cannot reach its `private` members
(`crates/nvs-types/src/signatures.rs:1741`). `#[Property]` is not a recognized attribute. The editor
finds `.nvst` cases with its own file search (`editors/vscode/src/tests.ts:143`), which is a second
implementation of discovery. And `nvs test` lacks what a team running a suite in CI expects: a time
limit per test, tags, shards, fail-fast, a rerun of the failures, a random order and a watch mode.
`mutation-testing` and `bench-counters` need stage 1, and `mutation-testing` needs stage 5.

## Standing decisions

**Four runner features join the test surface.** The user decided this: `nvs test --watch`, a per-test
time limit that reuses `[limits] wall_time` with a `#[Test(timeout: ...)]` override, `#[Test(tags:
[...])]` with `--tag`, `--skip-tag` and `--shard N/M`, and `--fail-fast`, `--rerun-failed` and a random
order with the seed printed. The four rules are `rule:testing/a-test-has-a-wall-time-limit`,
`rule:testing/tags-and-shards-select-what-runs`,
`rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle` and `rule:testing/watch-reruns-on-an-edit`.

**The details in those rules are settled.** A shard is a hash of the test's name, so it is stable
when tests are added. `--shuffle` is off by default and changes only the start order, never the
report's. `--rerun-failed` keeps its list in the data folder, never in the project. `--watch` reuses
the background check `nvs run` already has (`crates/nvs-cli/src/script.rs:1500`); there is no `nvs run
--watch` to reuse, and no second file watcher is built. `rule:testing/runner-is-strict` says what a
random start order can find.

**What counts as test code is settled in the rule before stage 1's code.**
`rule:testing/tests-never-reach-a-build` names a test method, its fixtures, its data rows and its
doubles. Stage 1 states which declarations that covers. A reference from other code into test code
is a compile error, because the same program must compile under `nvs run` and `nvs test`.

**An assertion outside a test records nothing.** The gap record this goal owns closes in stage 1:
`Ctx::record_assertion` keeps no entry when no test owns the context. Why: memory must be
O(in-flight), and a ledger nobody reads under `nvs run` grows with traffic.

**`#[Property]`'s feature id is `lang:testing/core-test-property-generated-inputs`.** Its folder is
`docs/examples/lang/testing/core-test-property-generated-inputs/`, and `chapter.md` names it the way
it names `core-test-testwith-data-rows`.

**Stage 5 runs right after stage 1.** Goals `mutation-testing` and `bench-counters` wait on stages 1
and 5 (`docs/goals/mutation-testing.md`, `docs/goals/bench-counters.md`), and stages 2 to 4 unblock
nothing. Stage ids stay as those goals cite them. Only the order changes.

**The editor test is the headless tier.** `editors/vscode/test/surfaces/tests.test.ts` runs under
`npm run test:headless` in `editors/vscode`, which starts no editor. Its recorded listing,
`editors/vscode/test/surfaces/recorded/list.json`, is re-recorded from the binary.

**Lanes.** Stages 1, 5, 6 and 9 share `crates/nvs-cli/src/runner.rs` or `main.rs`, so they run in the
order listed. Stage 2 (`signatures.rs`) and stages 3, 7 and 8 (`derive.rs`, then `runner.rs`'s
report, then `docs/examples/`) touch nothing stage 1 touches, and can run in any order beside it. Stage 4
(`--list` in `runner.rs`, and `editors/vscode/src/tests.ts`) shares `runner.rs` with 5 and 6, so it
runs between them or after them, never at once.

**Trade-offs, all together.** Request path: nothing, and stage 1 makes every built artifact smaller.
Compile path: one test-or-not flag per declaration, and a tag list per test. Memory under `nvs test`:
one deadline per test in flight, and under `--watch` the compiled program between passes. Under `nvs
run`, an assertion stops allocating. Usability: the features a CI pipeline uses, all off by default.
Simplicity: four new flags on `nvs test` and two new `#[Test]` options, each one named the way the
common runners name it.

## Stages

### 1. tests are absent from `nvs run` and `nvs build`

**Does:** the lowering leaves out every declaration that is test code, `nvs check` still checks it,
and an assertion outside a test keeps no ledger.

- [ ] The rule names what is test code, and that a reference into it is a compile error (`rule:testing/tests-never-reach-a-build`). `docs/rules/testing/tests-never-reach-a-build.md:1`
- [ ] The checker marks test code from its `#[Test]` table and the hooks. `crates/nvs-types/src/testing.rs:350`, `crates/nvs-types/src/check.rs:413`
- [ ] A reference from other code into test code is the compile error, with its code and card. `crates/nvs-types/src/testing.rs:350`
- [ ] `nvs run`, its reload and `nvs build` lower none of it, and `nvs test` lowers it as today. `crates/nvs-ir/src/lower/mod.rs:735`, `crates/nvs-ir/src/lower/mod.rs:776`, `crates/nvs-cli/src/main.rs:1626`, `crates/nvs-cli/src/script.rs:1407`, `crates/nvs-cli/src/runner.rs:932`
- [ ] Tests: a run's lowered program has no test label, and `nvs check` still reports an error in a test method. `crates/nvs-ir/src/lower/mod.rs:735`
- [ ] The rule `shipped`. `data/rules/testing/tests-never-reach-a-build.json:3`

The assertion ledger, the one gap record this goal owns:

- [ ] `Ctx::record_assertion` keeps no entry when no test owns the context. `crates/nvs-runtime/src/ctx/error.rs:782`
- [ ] A ladder ramps the assertion count under `nvs run` and pins flat memory. `benches/scaling/values/assertions-outside-a-test.nvs:1`
- [ ] The gap record deleted. `data/gaps/nvs-runtime/an-assertion-outside-a-test-grows-the-ledger.json:1`

### 5. a test has a time limit, and tags and shards select what runs

**Does:** two new `#[Test]` options and three new flags, as their rules say.

**Split:** after the flags land, one writing subagent extends the `lang:testing/test-options`
example with `timeout` and `tags`.

- [ ] `timeout` and `tags` join `#[Test]`'s option roster, and `timeout` is checked as a duration while compiling. `crates/nvs-types/src/testing.rs:231`
- [ ] Each test runs under `[limits] wall_time` or its own `timeout`, and a test past it fails with a message naming the limit (`rule:testing/a-test-has-a-wall-time-limit`). `crates/nvs-cli/src/runner.rs:490`
- [ ] `--tag`, `--skip-tag` and `--shard N/M`, and the summary counts what the selection left out (`rule:testing/tags-and-shards-select-what-runs`). `crates/nvs-cli/src/runner.rs:236`
- [ ] Tests for each, and the help text in the binary. `crates/nvs-cli/src/runner.rs:236`
- [ ] The example. `docs/examples/lang/testing/test-options/about.md:1`
- [ ] Both rules `shipped`. `data/rules/testing/a-test-has-a-wall-time-limit.json:3`, `data/rules/testing/tags-and-shards-select-what-runs.json:3`

### 2. a test reaches `private` members of classes in its own file

**Does:** visibility admits a test method that is declared in the same file as the member's class.

- [ ] The visibility check knows whether the accessing method is a test and which file declares it. `crates/nvs-types/src/signatures.rs:1741`
- [ ] A test in the same file reaches `private` and `protected` members, and a test in another file does not (`rule:testing/private-in-the-same-file`). `crates/nvs-types/src/signatures.rs:1741`
- [ ] The same access from a non-test method in that file is still refused. `crates/nvs-types/src/signatures.rs:1741`
- [ ] Tests for all three. `crates/nvs-types/src/signatures.rs:1741`
- [ ] The rule `shipped`. `data/rules/testing/private-in-the-same-file.json:3`

### 3. `#[Property]` is recognized, and its inputs are generated from the declared types

**Does:** a recognized attribute whose parameters get generators derived from their declared types.

- [ ] `#[Property]` joins the recognized roster with its card and its options, `gen:` among them. `crates/nvs-types/src/derive.rs:73`, `crates/nvs-types/src/derive.rs:191`, `crates/nvs-types/src/testing.rs:231`
- [ ] Generators come from the declared types by the walk a codec uses, user classes included, and a generated `string` includes combining marks and grapheme clusters (`rule:testing/property-testing`). `crates/nvs-types/src/derive.rs:749`
- [ ] Rust tests: a generator per scalar, a user class, a `string` with a grapheme cluster, a fixed seed giving the same inputs. `crates/nvs-types/src/derive.rs:749`

### 7. a `#[Property]` failure shrinks, and the report carries the counterexample and the seed

**Does:** the runner runs a property, shrinks a failure, and reports it.

- [ ] The runner runs each property over its generated inputs, as one test. `crates/nvs-cli/src/runner.rs:490`
- [ ] A failure shrinks to a minimal counterexample and reports the seed that reproduces it (`rule:testing/property-testing`). `crates/nvs-cli/src/runner.rs:490`
- [ ] The JSON report gains the counterexample key it is waiting for (`rule:testing/report-formats`). `crates/nvs-cli/src/runner.rs:2817`
- [ ] Novis tests: a property that holds, one that fails and shrinks, a seed that reproduces it. `docs/examples/lang/testing/core-test-property-generated-inputs/01-a-property-holds-for-every-input.nvs:1`

### 8. `#[Property]` has its feature proofs

**Does:** the feature owes nothing to `bun nv proofs`, and its rule ships.

**Split:** writing subagents, one after another: the three examples with `about.md`, then the bench,
then the attack. Each is held to `AGENTS.md` § *Text an end user reads*.

- [ ] `chapter.md` names the feature. `docs/examples/lang/testing/chapter.md:1`
- [ ] `about.md` and three examples. `docs/examples/lang/testing/core-test-property-generated-inputs/about.md:1`
- [ ] One bench with its growth. `benches/members/lang/testing/core-test-property-generated-inputs.nvs:1`
- [ ] One attack: a generator asked for a huge input stays under the test's limits. `tests/hostile/lang/testing/property-generator-size.nvs:1`
- [ ] `bun nv proofs --id 'lang:testing/core-test-property-generated-inputs'` passes (`rule:testing/feature-proofs`). `docs/examples/lang/testing/core-test-property-generated-inputs/about.md:1`
- [ ] The rule `shipped`. `data/rules/testing/property-testing.json:3`

### 4. `nvs test --list` covers `.nvst` trees, and the editor stops searching for them

**Does:** `--list` over a case tree lists its cases, and the VS Code client fills its tree from that
document alone.

- [ ] `--list` accepts a `.nvst` tree and lists each case with its path, in the JSON form the program listing uses. `crates/nvs-cli/src/runner.rs:2935`, `crates/nvs-cli/src/runner.rs:2963`
- [ ] The client drops its `findFiles` search for cases and reads the listing (`rule:ide/the-test-tree-is-discovered-and-run-through-the-cli`). `editors/vscode/src/tests.ts:77`, `editors/vscode/src/tests.ts:143`
- [ ] A CLI test over a case tree. `crates/nvs-cli/src/runner.rs:2935`
- [ ] The headless surface test and its recorded listing updated. `editors/vscode/test/surfaces/tests.test.ts:1`

### 6. fail-fast, rerun-failed and shuffle

**Does:** three more flags, as their rule says.

- [ ] `--fail-fast` stops starting tests after the first failure, and the summary counts what did not run (`rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`). `crates/nvs-cli/src/runner.rs:236`, `crates/nvs-cli/src/runner.rs:490`
- [ ] Each run writes its failed tests to the data folder, and `--rerun-failed` runs them. `crates/nvs-config/src/data.rs:225`
- [ ] `--shuffle` and `--shuffle=<seed>`, with the seed in the summary and the report order unchanged. `crates/nvs-cli/src/runner.rs:490`
- [ ] Tests for each, and the help text. `crates/nvs-cli/src/runner.rs:236`
- [ ] The `tools:cli/nvs-test` example shows the three flags. `docs/examples/tools/cli/nvs-test/about.md:1`
- [ ] The rule `shipped`. `data/rules/testing/a-run-can-stop-early-rerun-failures-and-shuffle.json:3`

### 9. `nvs test --watch`

**Does:** `--watch` reruns on each change the background check sees.

- [ ] `--watch` drives the runner from the background check's change signal (`rule:testing/watch-reruns-on-an-edit`). `crates/nvs-cli/src/script.rs:1500`, `crates/nvs-cli/src/runner.rs:261`
- [ ] A test: an edit to a watched file starts a second pass. `crates/nvs-cli/src/script.rs:1500`
- [ ] The help text. `crates/nvs-cli/src/runner.rs:236`
- [ ] The rule `shipped`, and `bun nv owners --closes test-surface` passes. `data/rules/testing/watch-reruns-on-an-edit.json:3`

## Handoff

### State

No stage is started. The anchors above were checked against the tree. Goals `mutation-testing` and
`bench-counters` gate on stages 1 and 5 by id, so those ids never change.

### Next

**Stage 1: tests out of builds**, one file set: `crates/nvs-ir/src/lower/mod.rs`,
`crates/nvs-types/src/testing.rs`, `crates/nvs-types/src/check.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-cli/src/script.rs`, and `crates/nvs-runtime/src/ctx/error.rs` for the gap. No gate.

- [ ] What is test code, in the rule (`rule:testing/tests-never-reach-a-build`). `docs/rules/testing/tests-never-reach-a-build.md:1`
- [ ] The checker marks it. `crates/nvs-types/src/testing.rs:350`, `crates/nvs-types/src/check.rs:413`
- [ ] A reference into test code is a compile error. `crates/nvs-types/src/testing.rs:350`
- [ ] `nvs run`, its reload and `nvs build` lower none of it. `crates/nvs-ir/src/lower/mod.rs:776`, `crates/nvs-cli/src/main.rs:1626`, `crates/nvs-cli/src/script.rs:1407`
- [ ] Tests: no test label in a run's program, `nvs check` still checks tests. `crates/nvs-ir/src/lower/mod.rs:735`
- [ ] The rule `shipped`. `data/rules/testing/tests-never-reach-a-build.json:3`
- [ ] `record_assertion` keeps nothing outside a test. `crates/nvs-runtime/src/ctx/error.rs:782`
- [ ] The flat-memory ladder. `benches/scaling/values/assertions-outside-a-test.nvs:1`
- [ ] The gap record deleted. `data/gaps/nvs-runtime/an-assertion-outside-a-test-grows-the-ledger.json:1`

### Backlog

- Shards balanced by recorded run time were declined (`docs/rules/testing/tags-and-shards-select-what-runs.md`).
- Goals `mutation-testing` and `bench-counters` gate on stages 1 and 5 (`docs/goals/mutation-testing.md`, `docs/goals/bench-counters.md`).
