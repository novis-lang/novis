---
looper: 1
title: "a probe drives every Core member past a small memory ceiling, and every member it finds past the bound is fixed"
summary: "bun nv proofs --probe exits 0 over every Core member it can call, and Rust tests in a debug build run under the default memory ceiling"
context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/abi.rs, tools/nv/proofs/run.ts]
stages:
  - id: 1
    title: "a Rust test in a debug build runs under the default memory ceiling"
    status: done
    context: [crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-config/src/defaults.rs]
  - id: 2
    title: "bun nv proofs --probe drives every static Core member past a 16 MiB ceiling and judges each reading"
    status: done
    context: [tools/nv/cmd/proofs.ts, crates/nvs-stdlib/src/registry.rs, crates/nvs-cli/tests/meta.rs]
  - id: 3
    title: "the probe reaches instance members through each class's own factory"
    status: open
    context: [tools/nv/cmd/proofs.ts, crates/nvs-stdlib/src/registry.rs]
  - id: 4
    title: "every member the probe or the test ceiling found past the bound is fixed and has an attack"
    status: open
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: test-ceiling
    stage: 1
    name: a context a Rust test makes in a debug build is under the default memory ceiling
    run: [cargo, test, --lib, a_test_context_is_under_the_default_memory_ceiling]
    tests:
      - a_test_context_is_under_the_default_memory_ceiling
  - id: probe-judge
    stage: 2
    name: the probe writes a program per member with three input shapes, and judges a reading past the bound as red
    run: [bun, test, tools/nv/test/probe.test.ts]
    tests:
      - a member gets a flat, a shared and a doubling input at four sizes
      - a callback parameter gets a function that returns its argument
      - a member of an excluded class is listed as skipped with its reason
      - a reading past the bound is red and names the member, the shape and the size
  - id: no-probe-overshoot
    stage: 4
    name: every Core member the probe can call stays within 1.5x of its ceiling plus 8 MiB
    run: [bun, nv, proofs, --probe]
    timeout: 3600
    reach: true
    floor: true
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, memory-overshoot-net]
    reach: true
    floor: false
---

# a probe drives every Core member past a small memory ceiling, and every member it finds past the bound is fixed

## Why

`memory-ceiling-holes` fixed every member its scan and review found past its ceiling. The class can
still come back: the allocator counts every byte but refuses none, so a native site that grows its own
`Vec`, `String` or `HashMap` is unbounded until the call returns. Today only an attack written for that
member finds it, and the debug backstop (`overshoot_backstop`, `crates/nvs-runtime/src/abi.rs:600`) only
runs in a test that sets a ceiling. Rust tests make their context with `Ctx::buffered` (818 sites) or
`Ctx::new` (397), and both arm no ceiling (`crates/nvs-runtime/src/ctx/wiring.rs:162`).

This goal builds the net that finds such a site without anyone writing an attack for it, and fixes what
the net finds. **It is also the evidence for a larger decision that is not this goal's**: whether every
growth in `Core` code should go through budget-checked wrapper types with a Clippy lint forcing them.
The user's call (2026-10-31): that refactor is taken only if the evidence shows the upside clearly
outweighs its cost. The count of members this goal finds and fixes is that evidence.

## Standing decisions

**The refactor is out of scope.** No wrapper type for `Vec`, `String` or `HashMap`, and no lint. A
finding is fixed at its site the way `memory-ceiling-holes` fixed its members: through `NvsArray`,
`ByteWriter`, `StrWriter` or `affords`. The handoff's `### State` keeps the findings table (member,
shape, size, reading, fix commit), so the retired file in git history is the evidence.

**The test ceiling is the shipped default.** In a `debug_assertions` build, `Ctx::new` arms
`LIMITS_MEMORY` (128 MiB, `crates/nvs-config/src/defaults.rs:60`), the limit a request gets when its
configuration sets none. `set_config` and `set_memory_limit` replace it as today, so `nvs test`, a
served request and the conformance runs keep their configured limit. A release build keeps `0`. A test
that needs more memory, or none, calls `set_memory_limit` itself with a comment saying why. That is a
change to the test, never a raised default. Cost: none in a release build. Safe fallback, if a debug
path that is not a test breaks under the ceiling (the LSP, `nvs code`, compile-time evaluation): a
`test-ceiling` cargo feature on `nvs-runtime`, enabled through each crate's dev-dependencies, arms it
instead of `debug_assertions`.

**A context that serves no request calls `Ctx::clear_memory_ceiling`.** `set_memory_limit(0)` is a
request under no ceiling. It arms the context's word, so the server's own memory limit can still stop
it. Work with no request arms nothing, and a debug build's `Ctx::new` no longer starts that way. So
`clear_memory_ceiling` arms `Armed::NONE` and sets the limit to `0`. Declined: a separate unarmed
constructor, which every caller of `Ctx::new` would have to choose between. Cost: one method, and
nothing on the request path.

**The probe uses the overshoot judge, not the backstop.** `bun nv proofs --probe` writes programs and
runs them on the proof binary through `spawnProof`, the way `--overshoot` does
(`tools/nv/proofs/run.ts:976`). A reading past `OVERSHOOT_FACTOR x ceiling + OVERSHOOT_SLACK_BYTES`
(`tools/nv/proofs/run.ts:275`) is red, and so is a program the host killed. The bound is never raised to
make a member pass.

**What a probe program is.** One program per member, under `.agent-tmp/probe/`, deleted when the run
ends.
- It lowers its own memory limit to 16 MiB with `Core\Config::set` first.
- It calls the member at four input sizes: 64 KiB, 512 KiB, 2 MiB and 8 MiB.
- At each size it tries three shapes for every string, list or map parameter:
  - **flat**: one value of that size
  - **shared**: a list that holds the same 64 KiB row many times, up to that size
  - **doubling**: `$a = [$a, $a]` repeated until the graph is that size if copied
- An integer parameter takes `0`, `1`, the size and the largest value its type allows.
- A callback parameter gets a function that returns its argument.
- A parameter of any other type takes the type's smallest value, read from the type in `nvs meta --json`.
- A catchable throw is a pass: the member refused. A finished call is a pass. Only the reading judges.

With a 16 MiB ceiling the bound is 32 MiB, so an 8 MiB input finds any member that grows its input
more than four times.

**The probe skips what it cannot call safely, and says so.** It skips members that touch files,
processes, the network, a database, a directory server, mail, a shared store, or that sleep:
`Core\IO`, `Core\Process`, `Core\Http`, `Core\Mail`, `Core\Db`, `Core\Ldap`, `Core\Queue`,
`Core\Cache`, `Core\Topic` and `Core\Time::sleep`, plus each class an `Explore` subagent finds of
the same kind in stage 2. Each skip prints its reason. `memory-ceiling-holes`'s stage 7, 8 and 9 attacks
cover those readers. The skip list is one constant in the command, with its reason beside it.

**The source of members is `nvs meta --json`.** It publishes every member's names and parameter types
from `CLASSES` (`crates/nvs-stdlib/src/registry.rs:1208`). The probe reads it from the proof binary, so
the probe and the binary always agree.

**How the probe reads what it cannot judge.** Stage 2 settled these while it built the command
(`tools/nv/proofs/probe.ts`).
- **A within-bound stop runs again.** A memory `FATAL` ends the program, so a stop at a small input
  would hide every larger one. A run that a limit stopped within the bound runs again from the next
  step, and the member's row is its worst run. Cost: one more process per stop.
- **A shape runs only where a parameter can take it.** A string or bytes parameter takes flat alone. A
  list or `mixed` parameter takes shared, and a `mixed` or `array<mixed>` one takes doubling. A generic
  `T` is written `mixed`, and a member that takes one type argument gets `array<{ a: string }>`.
- **What is not a reading is listed, never red.** A parameter the probe cannot build (an instance, a
  shape, a numeric list) is "not built", with its type. A program that does not compile is listed with
  its first error. A runtime crash (`panicked at`, `internal error`, `stack overflow`) is red.
- **Only `Core\` is probed.** The `Novis\` component classes `nvs meta --json` also lists are counted
  and not probed; their memory is the component host's.

**Two at a time, like `--overshoot`.** The full run is a floor check with a 3600 s timeout. Stage 2
records how long it took. If it takes more than 30 minutes, the sizes drop to the three largest and
the shapes to the ones a member's parameters can take.

**Lanes.** Stages 1 and 2 touch disjoint files and may run in either order or side by side. Stage 3
needs stage 2's command. Stage 4 needs the findings of stages 1 to 3. Its fixes touch member files,
which are disjoint per class, so its fixes may go to writing subagents one class each.

**Trade-offs.**
- Security: finds the members that can still hold more than their ceiling inside one call.
- Latency: nothing on the request path changes. The fixes ask the budget at growth, one comparison
  per reallocation.
- Memory: nothing is added.
- Simplicity: one command and one constant. Tests that need a large heap say so.

## Stages

### 1. a Rust test in a debug build runs under the default memory ceiling

**Does:** every Rust test that makes a context runs the debug backstop, so a member past its bound
panics in the test that called it.

- [x] `Ctx::new` arms `LIMITS_MEMORY` in a `debug_assertions` build, and its doc says why and what a
  test that needs more does. `crates/nvs-runtime/src/ctx/wiring.rs:103`, `crates/nvs-runtime/src/ctx/wiring.rs:162`
- [x] The test `a_test_context_is_under_the_default_memory_ceiling` in wiring's test module. Run once
  by name. `crates/nvs-runtime/src/ctx/wiring.rs:103`
- [x] Run `cargo test` once over the tree. This run is the stage's requirement, because it is the
  finding. Sort its failures into two lists:
  - **backstop panics**: a member passed the bound. Each goes to stage 4's findings table.
  - **tests that need a large heap on purpose**: each calls `set_memory_limit` with its reason.
- [x] The budget module doc names the test ceiling beside the backstop. `crates/nvs-runtime/src/budget.rs:233`

### 2. bun nv proofs --probe drives every static Core member past a 16 MiB ceiling and judges each reading

**Split:** an `Explore` subagent lists every class in `nvs meta --json` whose members touch files,
processes, the network, a store or the clock, for the skip list.

- [x] `bun nv proofs --probe` reads `nvs meta --json` from the proof binary and writes one program per
  static member, by the standing decision. `tools/nv/proofs/run.ts:976`, `tools/nv/cmd/proofs.ts:15`
- [x] It runs them two at a time through `spawnProof` and judges each reading with the overshoot
  bound. It prints the worst first, then the skips with their reasons. Exit 1 on a reading past the
  bound or a killed program. `tools/nv/proofs/run.ts:888`
- [x] `USAGE`, the flag list and the module doc name `--probe`. `tools/nv/cmd/proofs.ts:70`, `tools/nv/cmd/proofs.ts:445`
- [x] The four tests the `probe-judge` check names. Run once by name: `bun test tools/nv/test/probe.test.ts`. `tools/nv/test/probe.test.ts:1`
- [x] Run `bun nv proofs --probe` once. Each red reading goes to stage 4's findings table. Record the
  run's length in the handoff.

### 3. the probe reaches instance members through each class's own factory

**Split:** an `Explore` subagent lists each class with instance members and the static member or
constructor that makes one from plain values.

- [ ] The probe makes a receiver from each class's factory with the smallest arguments, then calls each
  instance member with the standing decision's sizes and shapes. A class with no factory the probe can
  call is a skip with its reason. `tools/nv/proofs/run.ts:976`
- [ ] A test in `tools/nv/test/probe.test.ts` that an instance member gets a receiver from its factory. Run once by name.
- [ ] Run `bun nv proofs --probe` once. Each new red reading goes to stage 4's findings table.

### 4. every member the probe or the test ceiling found past the bound is fixed and has an attack

**Does:** the findings table is listed here by stages 1 to 3, one item per member, before this stage
starts. Each fix goes through `NvsArray`, `ByteWriter`, `StrWriter` or `affords`, and each fixed member
gets an attack step that ends at `FATAL` within the bound.

**Split:** one writing subagent per class, side by side. The session builds once they all end.

- [ ] The findings table: filled in by stages 1 to 3. Stage 2's, from `bun nv proofs --probe` on the
  proof binary of `9925cba9a` (input is the step's flat string, ceiling 16 MiB, bound 32 MiB):
  - [ ] `Core\Regex::compile`: flat 8 MiB pattern, 731.2 MiB (45.70x).
  - [ ] `Core\Regex::match`, `matches`, `matchAll`, `split`, `replaceWith`: flat 2 MiB, 193.3 MiB
    (12.08x) each. The pattern is the same 2 MiB string, so the compile is the likely site.
  - [ ] `Core\Regex::replace`: flat 512 KiB, 72.3 MiB (4.52x).
  - [ ] `Core\Metrics::increment`, `gauge`, `observe`: flat 8 MiB name, 37.2 MiB (2.32x).
  - [ ] `Core\BigInt::parse`, `Core\Time\Zone::of`, `Core\Debug::startTrace`, `startProfile`,
    `Core\Command::help`, `Core\Cldr::pluralCategory`, `ordinalCategory`: flat 8 MiB, 32.0 MiB (2.00x),
    just past the bound. Each likely copies the input twice, such as into an error message.
- [ ] `bun nv proofs --probe` exits 0.

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done. Stage 2's command is `9925cba9a`: `tools/nv/proofs/probe.ts`, the flag in
`tools/nv/cmd/proofs.ts`, and `tools/nv/test/probe.test.ts` (its four tests pass, run once by name).
The decisions it settled are under standing decisions, in *How the probe reads what it cannot judge*.

The full run, `bun nv proofs --probe --nvs target/proof/nvs.exe`, **took 63 s**, far under the 30
minute line, so the sizes and shapes stay as decided. Of 690 static `Core` members it ran 369, skipped
164 by `PROBE_SKIPS` and did not build 42. **17 are red**, and they are stage 4's findings table, listed
under stage 4. 198 finished every step, and 141 stopped at a memory limit within the bound. The worst
green ones are `Core\Uri::buildQuery`, `Core\Csv::parse` and `Core\Path::fromCwd` at 1.50x.

Thirteen programs did not compile, and are listed, not red. `Core\Arr::fillKeys`, `fromKeysAndValues`
and `from` need a typed list. `Core\Math::abs` needs a number for its bounded `T`. The `*As` members
and `Core\Attributes` reject the `array<{ a: string }>` type argument. `Core\Path::thisDir` and
`Core\Test::assertThrows` need a literal. They need no change for the goal.

The acceptance sweep's reds are not tree faults. `env-secret-proofs` has no definition in the tree. It
is stale state in looper's service. The nightly run 41187571578 ran on `857b39cad`, before the fixes
`39795c9c4`, `bfce9c2d2`, `593b6c565` and `8ce92dda5`.

The `no-probe-overshoot` check is red until stage 4 fixes the 17. No other test is expected to go red.

### Next

Stage 3: instance members through each class's factory. The probe is `tools/nv/proofs/probe.ts`. Its
planner is `probeProgram` and `planParam`, and `probePlan` keeps `kind: "static"` alone today. A
receiver needs a factory per class. Start with the split's `Explore` list of each class with instance
members and the static member or constructor that makes one from plain values. The 42 "not built"
members (an instance, a `Core\Time\Duration`, a `Core\Charset`, a numeric list) can reuse the same
factory table as arguments. Run `bun nv proofs --probe --only 'Core\X'` to try one class in seconds.

Then stage 4, one writing subagent per class: Regex first (45.70x), then Metrics, then the seven at 2.00x.

### Backlog
