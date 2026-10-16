---
looper: 1
title: Tests stay out of builds, and the test runner has the features a team expects
summary: "No test reaches nvs run or nvs build, a test reaches private members in its own file, #[Property] runs, nvs test --list covers .nvst trees, the time limit, tags, shards, fail-fast, rerun-failed, shuffle and watch work, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs, crates/nvs-types/src/testing.rs]
stages:
  - id: 1
    title: tests are absent from `nvs run` and `nvs build`
    status: open
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-cli/src/main.rs, crates/nvs-types/src/testing.rs, crates/nvs-types/src/check.rs, docs/rules/testing/tests-never-reach-a-build.md]
    extra:
      context:
        rules: [testing/tests-never-reach-a-build, testing/test-attribute, testing/fixtures, testing/class-hooks]
  - id: 2
    title: a test reaches `private` members of classes in its own file
    status: open
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/testing.rs, docs/rules/testing/private-in-the-same-file.md]
    extra:
      context:
        rules: [testing/private-in-the-same-file, testing/tests-never-reach-a-build]
  - id: 3
    title: "`#[Property]` runs a method against generated inputs"
    status: open
    context: [crates/nvs-types/src/derive.rs, crates/nvs-types/src/testing.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/property-testing.md]
    extra:
      context:
        rules: [testing/property-testing, testing/data-rows, testing/report-formats, testing/determinism-declared-on-the-test]
  - id: 4
    title: "`nvs test --list` covers `.nvst` trees, and the editor stops searching for them"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs, editors/vscode/src/tests.ts, docs/rules/ide/the-test-tree-is-discovered-and-run-through-the-cli.md]
    extra:
      context:
        rules: [ide/the-test-tree-is-discovered-and-run-through-the-cli, testing/report-formats, testing/nvst-is-separate]
  - id: 5
    title: a test has a time limit, and tags and shards select what runs
    status: open
    context: [crates/nvs-types/src/testing.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs, docs/rules/testing/a-test-has-a-wall-time-limit.md, docs/rules/testing/tags-and-shards-select-what-runs.md]
    extra:
      context:
        rules: [testing/a-test-has-a-wall-time-limit, testing/tags-and-shards-select-what-runs, testing/test-attribute, errors/on-limit]
  - id: 6
    title: fail-fast, rerun-failed, shuffle and watch
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs, crates/nvs-cli/src/script.rs, crates/nvs-config/src/data.rs, docs/rules/testing/a-run-can-stop-early-rerun-failures-and-shuffle.md, docs/rules/testing/watch-reruns-on-an-edit.md]
    extra:
      context:
        rules: [testing/a-run-can-stop-early-rerun-failures-and-shuffle, testing/watch-reruns-on-an-edit, testing/runner-is-strict, config/an-edit-reaches-the-next-request-without-a-restart]
checks:
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, test-surface]
    reach: true
    floor: false
extra:
  milestone: M10
---

# Tests stay out of builds, and the test runner has the features a team expects

## Why

Three designed testing rules are not on disk. `nvs run` and `nvs build` lower every `#[Test]` method
into the artifact (`crates/nvs-ir/src/lower/mod.rs:774`), so production carries the test code its rule
says it never sees. A test in the same file as a class cannot reach its `private` members
(`crates/nvs-types/src/signatures.rs:1733`). `#[Property]` is not a recognized attribute. The editor
finds `.nvst` cases with its own file search (`editors/vscode/src/tests.ts:77`), which is a second
implementation of discovery. And `nvs test` lacks what a team running a suite in CI expects: a time
limit per test, tags, shards, fail-fast, a rerun of the failures, a random order and a watch mode.
`mutation-testing` needs stage 1, because a mutant must never be made from test code.

## Standing decisions

**Four runner features join the test surface.** The user decided this: `nvs test --watch`, a per-test
time limit that reuses `[limits] wall_time` with a `#[Test(timeout: ...)]` override, `#[Test(tags:
[...])]` with `--tag`, `--skip-tag` and `--shard N/M`, and `--fail-fast`, `--rerun-failed` and a random
order with the seed printed. The four rules are `rule:testing/a-test-has-a-wall-time-limit`,
`rule:testing/tags-and-shards-select-what-runs`,
`rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle` and `rule:testing/watch-reruns-on-an-edit`.

**The details in those rules are the planning session's calls.** A shard is a hash of the test's
name, so it is stable when tests are added. `--shuffle` is off by default and changes only the start
order, never the report's. `--rerun-failed` keeps its list in the data folder, never in the project.
`--watch` reuses the background check `nvs run` already has (`crates/nvs-cli/src/script.rs:1501`); there
is no `nvs run --watch` to reuse, and no second file watcher is built. `rule:testing/runner-is-strict`
says what a random start order can find.

**What counts as test code is settled in the rule before stage 1's code.** The planning session's
call. `rule:testing/tests-never-reach-a-build` names a test method, its fixtures, its data rows and its
doubles. Stage 1 states which declarations that covers and what a reference from other code to test
code does. A compile error is the safe default, because the same program must compile under `nvs run`
and `nvs test`.

**Trade-offs, all together.** Request path: nothing, and stage 1 makes every built artifact smaller.
Compile path: one test-or-not flag per declaration, and a tag list per test. Memory under `nvs test`:
one deadline per test in flight, and under `--watch` the compiled program between passes. Usability:
the features a CI pipeline uses, all off by default. Simplicity: four new flags on `nvs test` and two
new `#[Test]` options, each one named the way the common runners name it.

## Stages

### 1. tests are absent from `nvs run` and `nvs build`

**Does:** the lowering leaves out every declaration that is test code, and `nvs check` still checks it.

- [ ] The rule names what is test code, and what a reference from other code into it does (`rule:testing/tests-never-reach-a-build`). `docs/rules/testing/tests-never-reach-a-build.md`
- [ ] The checker marks test code from its `#[Test]` table and the hooks. `crates/nvs-types/src/testing.rs:241`, `crates/nvs-types/src/check.rs:413`
- [ ] `nvs run` and `nvs build` lower none of it, and `nvs test` lowers it as today. `crates/nvs-ir/src/lower/mod.rs:774-784`, `crates/nvs-cli/src/main.rs:2738`, `crates/nvs-cli/src/main.rs:3128`
- [ ] A test that a built program has no test label in its IR, a test that `nvs check` still reports an error in a test method, and the rule `shipped`.

### 2. a test reaches `private` members of classes in its own file

**Does:** visibility admits a test method that is declared in the same file as the member's class.

- [ ] The visibility check knows whether the accessing method is a test and which file declares it. `crates/nvs-types/src/signatures.rs:1733-1745`
- [ ] A test in the same file reaches `private` and `protected` members, and a test in another file does not (`rule:testing/private-in-the-same-file`).
- [ ] The same access from a non-test method in that file is still refused. Tests for all three, and the rule `shipped`.

### 3. `#[Property]` runs a method against generated inputs

**Does:** a recognized attribute whose parameters get generators derived from their declared types,
with shrinking and a seed.

- [ ] `#[Property]` joins the recognized roster with its card and its options, `gen:` among them. `crates/nvs-types/src/derive.rs:73`, `crates/nvs-types/src/derive.rs:191`
- [ ] Generators come from the declared types by the walk a codec uses, user classes included, and a generated `string` includes combining marks and grapheme clusters (`rule:testing/property-testing`). `crates/nvs-types/src/derive.rs:750`
- [ ] A failure shrinks to a minimal counterexample and reports the seed that reproduces it. The JSON report gains the counterexample key it is waiting for. `crates/nvs-cli/src/runner.rs:2628-2631`
- [ ] Tests from Novis and from Rust, and the feature's proofs: `bun nv proofs --id` once the roster names it. The rule `shipped`.

### 4. `nvs test --list` covers `.nvst` trees, and the editor stops searching for them

**Does:** `--list` over a case tree lists its cases, and the VS Code client fills its tree from that
document alone.

- [ ] `--list` accepts a `.nvst` tree and lists each case with its path, in the JSON form the program listing uses. `crates/nvs-cli/src/main.rs:645-657`
- [ ] The client drops its `findFiles` search for cases and reads the listing (`rule:ide/the-test-tree-is-discovered-and-run-through-the-cli`). `editors/vscode/src/tests.ts:76-78`, `editors/vscode/src/tests.ts:143-150`
- [ ] A CLI test over a case tree, and the client's surface test updated. `editors/vscode/test/surfaces/tests.test.ts`

### 5. a test has a time limit, and tags and shards select what runs

**Does:** two new `#[Test]` options and three new flags, as their rules say.

- [ ] `timeout` and `tags` join `#[Test]`'s option roster, and `timeout` is checked as a duration while compiling. `crates/nvs-types/src/testing.rs:231-239`
- [ ] Each test runs under `[limits] wall_time` or its own `timeout`, and a test past it fails with a message naming the limit (`rule:testing/a-test-has-a-wall-time-limit`).
- [ ] `--tag`, `--skip-tag` and `--shard N/M`, and the summary counts what the selection left out (`rule:testing/tags-and-shards-select-what-runs`). `crates/nvs-cli/src/main.rs:574`
- [ ] Tests, the help text in the binary, and both rules `shipped`.

### 6. fail-fast, rerun-failed, shuffle and watch

**Does:** four more flags, as their rules say.

- [ ] `--fail-fast` stops starting tests after the first failure, and the summary counts what did not run (`rule:testing/a-run-can-stop-early-rerun-failures-and-shuffle`). `crates/nvs-cli/src/main.rs:574`
- [ ] Each run writes its failed tests to the data folder, and `--rerun-failed` runs them. `crates/nvs-config/src/data.rs:119`
- [ ] `--shuffle` and `--shuffle=<seed>`, with the seed in the summary and the report order unchanged.
- [ ] `--watch` reruns on each change the background check sees (`rule:testing/watch-reruns-on-an-edit`). `crates/nvs-cli/src/script.rs:1501`
- [ ] Tests, the help text, examples for `tools:cli/nvs-test` and `lang:testing/test-options`, both rules `shipped`, and `bun nv owners --closes test-surface` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, tests out of builds, on `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-types/src/testing.rs`
and `crates/nvs-cli/src/main.rs`:
- settle what is test code in `rule:testing/tests-never-reach-a-build`.
- mark it in the checker. `crates/nvs-types/src/testing.rs:241`
- leave it out of the lowering for `nvs run` and `nvs build`. `crates/nvs-ir/src/lower/mod.rs:774`

### Backlog

- Shards balanced by recorded run time were declined (`docs/rules/testing/tags-and-shards-select-what-runs.md`).
- Goal `mutation-testing` depends on stage 1 and on stage 5's time limit (`docs/rules/testing/mutation-testing.md`).
