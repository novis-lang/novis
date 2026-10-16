---
looper: 1
title: A test class prepares and cleans up around its tests
summary: "`#[BeforeClass]`, `#[AfterClass]` and `#[AfterEachTest]` run as their rules say, tests run across every core, a subclass inherits the hooks, the fixture examples show a real cost, and `bun nv owners --closes test-hooks` passes."
context: [crates/nvs-types/src/testing.rs, crates/nvs-cli/src/runner.rs, crates/nvs-types/src/derive.rs]
stages:
  - id: 1
    title: the rules
    status: done
    extra:
      context:
        rules: [testing/class-hooks, testing/after-each-test, testing/hook-inheritance, testing/constructor-is-setup]
        shapes: [A rule fragment]
  - id: 2
    title: the three attributes compile and are checked
    status: done
    context: [crates/nvs-types/src/testing.rs, crates/nvs-types/src/derive.rs]
    extra:
      context:
        rules: [testing/class-hooks, testing/after-each-test, testing/hook-inheritance, testing/test-attribute]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 3
    title: the runner calls the hooks, with `Outcome` and `Summary`
    status: done
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-stdlib/src/test.rs]
    extra:
      context:
        rules: [testing/class-hooks, testing/after-each-test, testing/runner-is-strict, testing/fixtures]
        shapes: ["A `.nvst` test case", "A `Core` member"]
  - id: 4
    title: tests run across every core
    status: done
    context: [crates/nvs-cli/src/runner.rs, crates/nvs-host/src/placed.rs]
    extra:
      context:
        rules: [testing/tests-run-across-every-core, testing/class-hooks, testing/runner-is-strict, concurrency/on-worker-runs-the-child-on-another-core]
  - id: 5
    title: a subclass inherits hooks, fixtures and tests
    status: done
    context: [crates/nvs-types/src/testing.rs, crates/nvs-cli/src/runner.rs]
    extra:
      context:
        rules: [testing/hook-inheritance, testing/fixtures]
        shapes: ["A `.nvst` test case"]
  - id: 6
    title: the hooks remove a real file and a real database
    status: open
    context: [crates/nvs-cli/src/runner.rs]
    extra:
      context:
        rules: [testing/after-each-test, testing/class-hooks, testing/db-transaction]
  - id: 7
    title: the feature proofs, and fixture examples with a real cost
    status: open
    context: [docs/examples/lang/testing/core-test-fixture-built-once-injected-by-type/about.md, website/src/content/docs/syntax/testing/fixtures-and-assertions.mdx]
    extra:
      context:
        rules: [testing/feature-proofs, testing/website-pages-are-stamped]
checks:
  - id: rules-check
    stage: 1
    name: the rulebook renders and every record is valid
    run: [bun, nv, rules, --check]
  - id: hooks-compile
    stage: 2
    name: each hook's shape is checked, and a second hook of one kind or a static write does not compile
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/a-test-class-has-at-most-one-hook-of-each-kind.nvst, --case, tests/conformance/reject/a-class-hook-does-not-write-a-static.nvst, --case, tests/conformance/reject/a-hook-has-the-shape-its-rule-gives.nvst]
    tests: [tests/conformance/reject/a-test-class-has-at-most-one-hook-of-each-kind.nvst, tests/conformance/reject/a-class-hook-does-not-write-a-static.nvst, tests/conformance/reject/a-hook-has-the-shape-its-rule-gives.nvst]
  - id: hooks-run
    stage: 3
    name: the hooks run in order, after a failure, after every attempt, and receive `Outcome` and `Summary`
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/class-hooks-run-once-around-a-class.nvst, --case, tests/conformance/lang/after-class-runs-when-before-class-throws.nvst, --case, tests/conformance/lang/after-each-test-runs-after-every-attempt.nvst]
    tests: [tests/conformance/lang/class-hooks-run-once-around-a-class.nvst, tests/conformance/lang/after-class-runs-when-before-class-throws.nvst, tests/conformance/lang/after-each-test-runs-after-every-attempt.nvst]
  - id: tests-parallel
    stage: 4
    name: tests run at once across cores, the class hooks keep their order, and the report and coverage match a run on one core
    run: [cargo, test, --bin, nvs, "runner::tests::parallel"]
    tests: [parallel_tests_overlap_on_more_than_one_core, parallel_class_hooks_wrap_every_test_of_their_class, parallel_report_is_in_declaration_order, parallel_coverage_is_each_tests_own, a_failed_tests_output_is_printed_under_its_failure, a_passed_tests_output_is_discarded]
  - id: hooks-inherit
    stage: 5
    name: a subclass runs an inherited hook, fixture and test under its own name, and an override without its attribute does not compile
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-subclass-inherits-hooks-and-fixtures.nvst, --case, tests/conformance/reject/an-override-of-a-hook-repeats-its-attribute.nvst]
    tests: [tests/conformance/lang/a-subclass-inherits-hooks-and-fixtures.nvst, tests/conformance/reject/an-override-of-a-hook-repeats-its-attribute.nvst]
  - id: owners-closes
    stage: 7
    name: no designed rule or gap names this goal
    run: [bun, nv, owners, --closes, test-hooks]
    reach: true
---

# A test class prepares and cleans up around its tests

## Why

A test class can build a value once for all its tests with `#[Fixture]`, but it has no way to prepare
something that is not a value, and no way to clean up afterwards. The common case is a database: create
it, build the schema and seed rows once for the class, then drop it after the last test, whether the
tests passed or not. A fixture cannot do that. It is built only when a test declares its type, and it
has no cleanup. Each test also runs in its own isolate, so nothing a test or a helper does is shared
with the next test. This goal adds the class-level setup and cleanup that isolation leaves missing, and
a per-test cleanup that knows which test ran.

## Standing decisions

The four rules are the home of every decision, agreed with the user: `rule:testing/class-hooks`,
`rule:testing/after-each-test`, `rule:testing/hook-inheritance` and
`rule:testing/tests-run-across-every-core`. This goal restates none of them.

**The parallel runner is built here, not in M10.** The user wants the hooks and the runner they are
ordered by finished together, so the gap that tracked it is gone and the rule owns what is not built.

**Nothing is left to ask the user.** Every design question was answered with the user before this goal
started, and the rules hold the answers: the output of a test running beside others, inherited tests
running per subclass, the hook order under parallel runs. A question that comes up while building is
answered from the rules and the priority ordering, and the answer goes into the rule it belongs to.

**The names are the user's.** `#[BeforeClass]`, `#[AfterClass]` and `#[AfterEachTest]`. `#[After]` was
renamed before it existed, so no program uses the old name.

**The fixture examples are part of this goal.** They build values so cheap that a reader cannot see why
a fixture exists. Example 03 is rewritten around setup that really costs time, and `about.md` gains one
sentence: each test runs on its own, so without a fixture this setup would run once per test.

**Trade-offs.** No cost on the request path, and none for a class that does not use the hooks. A test
run gains one `Outcome` per attempt and one `Summary` per class, both small and released when the hook
returns. The language gains three attributes, and the rejection of a class-level pair in
`rule:testing/constructor-is-setup` is reversed.

## Stages

### 1. the rules

**Does:** the four rules exist as `designed`, owned by this goal, and `rule:testing/constructor-is-setup` points at them.

- [x] `rule:testing/class-hooks`, `rule:testing/after-each-test`, `rule:testing/hook-inheritance`. `docs/rules/testing/class-hooks.md:1`
- [x] `#[After]` and its rule are gone, and the M10 plan no longer names it. `docs/plan/m10.md:56`

### 2. the three attributes compile and are checked

**Does:** the checker recognises the three attributes, checks each one's shape and parameter, refuses a second hook of one kind, and refuses a static write in a class hook's own body.

- [x] The three names on the roster. `crates/nvs-types/src/derive.rs:71`
- [x] The shape checks beside the test and fixture ones. `crates/nvs-types/src/testing.rs:345`

### 3. the runner calls the hooks, with `Outcome` and `Summary`

**Does:** the runner calls the hooks in the order and on the failures `rule:testing/class-hooks` gives, and `#[AfterEachTest]` after every attempt.

- [x] The class pair around the fixtures and tests. `crates/nvs-cli/src/runner.rs:804`
- [x] `#[AfterEachTest]` inside each attempt. `crates/nvs-cli/src/runner.rs:897`
- [x] `Core\Test\Outcome` and `Core\Test\Summary`. `crates/nvs-stdlib/src/test.rs:1`

### 4. tests run across every core

**Does:** `nvs test` runs test isolates at once across every core with a program's own placement under `--jobs`, keeps the order `rule:testing/class-hooks` gives, captures each test's output, and reports and counts coverage as a run on one core does.

- [x] `--jobs` reaches a `#[Test]` run, and its help no longer says the methods are not spread. `crates/nvs-cli/src/main.rs:583`
- [x] Each test's output captured, printed under a failure, discarded on a pass, `--show-output` to stream it, and end of input for a read; the module comment at the anchor rewritten. `crates/nvs-cli/src/runner.rs:16`
- [x] Place each test isolate with `on: "worker"`'s placement, the class hooks around each class's tests. `crates/nvs-cli/src/runner.rs:804`
- [x] Coverage counted into one table per test and added to the run's when it finishes. `crates/nvs-cli/src/runner.rs:378`
- [x] Each `#[Test(server: true)]` listener runs beside others. `crates/nvs-cli/src/runner.rs:1505`

### 5. a subclass inherits hooks, fixtures and tests

**Does:** test collection reads inherited methods, so an inherited hook, fixture or test runs for each subclass under the subclass's name, and an override without its attribute does not compile.

- [x] Collect a class's inherited members as well as its own. `crates/nvs-types/src/test_inherit.rs:1`
- [x] The runner reports an inherited test under the subclass. `crates/nvs-cli/src/runner.rs:888`

### 6. the hooks remove a real file and a real database

**Does:** a runner test shows a file removed by `#[AfterEachTest]` and a database seeded by `#[BeforeClass]` and dropped by `#[AfterClass]`, under the grants the program already gives a test.

- [ ] The database case over the compose PostgreSQL the `db:` runner tests already open. `crates/nvs-cli/src/runner.rs:2759`

### 7. the feature proofs, and fixture examples with a real cost

**Does:** the hooks have their feature proofs, and the fixture page shows why a fixture exists.

- [ ] `bun nv proofs --id` for each hook, and every proof it names. `docs/examples/lang/testing/core-test-fixture-built-once-injected-by-type/about.md:1`
- [ ] Example 03 rewritten around setup that costs time. `docs/examples/lang/testing/core-test-fixture-built-once-injected-by-type/03-setup-that-happens-once.nvs:1`
- [ ] The three rules flipped to `shipped` with their guard tests. `docs/rules/testing/class-hooks.md:1`

## Handoff

### State

Stages 2 to 5 are done, and `hooks-compile`, `hooks-run`, `tests-parallel` and `hooks-inherit`
are green. Inheritance is `crates/nvs-types/src/test_inherit.rs`, and its module doc is the home
of how it works. At check time it reads each ancestor's marked methods from the whole-program
signature and attribute tables (`AttributeTable::method_carries`). It refuses an override that
drops its attribute, and a second hook beside an inherited one. It also gives the class's own
parameters the inherited fixtures. After the file loop, `merge` builds each subclass's tables
and removes an abstract class's tests. An inherited static hook or fixture is called through
`Unit::call_static_as` with its `declared_in` class, and the subclass as the called class.

### Next

**Stage 6: the hooks remove a real file and a real database**, file set:
`crates/nvs-cli/src/runner.rs`.

- [ ] A runner test where `#[AfterEachTest]` removes the file each test wrote (`rule:testing/after-each-test`). `crates/nvs-cli/src/runner.rs:2393`
- [ ] A runner test where `#[BeforeClass]` seeds a table in the compose PostgreSQL and `#[AfterClass]` drops it (`rule:testing/class-hooks`). `crates/nvs-cli/src/runner.rs:3497`

### Backlog

- No runner test holds several `server: true` listeners open at once; the one-listener test passes
  on a worker core. `crates/nvs-cli/src/runner.rs:2016`
- An abstract base's `#[Test]` that needs a type only its subclasses supply does not compile in the
  base. An overridable fixture in the base is how to write that today. `crates/nvs-types/src/testing.rs:516`
- An inherited fixture that needs an own fixture of the subclass is not in the cycle check, which
  reads the class's own rows. `crates/nvs-types/src/testing.rs:@reject_fixture_cycles`
- The three rules still read `"status": "designed"`; stage 7 flips them.
  `data/rules/testing/class-hooks.json`
- The nightly CI dispatch runs on `origin/main`, which is far behind local `main` because the loop
  never pushes. Its failures repeat until the user pushes. `tools/nv/cmd/sweep.ts`
- Not checked at HEAD: the Linux watcher test
  (`crates/nvs-cli/tests/run_command.rs:687`), the one-to-four core scaling margin
  (`crates/nvs-cli/src/serve.rs:3957`, it failed on one OS and then the other, so it may be flaky)
  and the stale `perf:` proofs. `tools/nv/cmd/sweep.ts`
