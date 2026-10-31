---
looper: 1
title: "a probe drives every Core member past a small memory ceiling, and every member it finds past the bound is fixed"
summary: "bun nv proofs --probe exits 0 over every Core member it can call, and Rust tests in a debug build run under the default memory ceiling"
context: [crates/nvs-runtime/src/budget.rs, crates/nvs-runtime/src/abi.rs, tools/nv/proofs/run.ts]
stages:
  - id: 1
    title: "a Rust test in a debug build runs under the default memory ceiling"
    status: open
    context: [crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-config/src/defaults.rs]
  - id: 2
    title: "bun nv proofs --probe drives every static Core member past a 16 MiB ceiling and judges each reading"
    status: open
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

- [ ] `Ctx::new` arms `LIMITS_MEMORY` in a `debug_assertions` build, and its doc says why and what a
  test that needs more does. `crates/nvs-runtime/src/ctx/wiring.rs:103`, `crates/nvs-runtime/src/ctx/wiring.rs:162`
- [ ] The test `a_test_context_is_under_the_default_memory_ceiling` in wiring's test module. Run once
  by name. `crates/nvs-runtime/src/ctx/wiring.rs:103`
- [ ] Run `cargo test` once over the tree. This run is the stage's requirement, because it is the
  finding. Sort its failures into two lists:
  - **backstop panics**: a member passed the bound. Each goes to stage 4's findings table.
  - **tests that need a large heap on purpose**: each calls `set_memory_limit` with its reason.
- [ ] The budget module doc names the test ceiling beside the backstop. `crates/nvs-runtime/src/budget.rs:233`

### 2. bun nv proofs --probe drives every static Core member past a 16 MiB ceiling and judges each reading

**Split:** an `Explore` subagent lists every class in `nvs meta --json` whose members touch files,
processes, the network, a store or the clock, for the skip list.

- [ ] `bun nv proofs --probe` reads `nvs meta --json` from the proof binary and writes one program per
  static member, by the standing decision. `tools/nv/proofs/run.ts:976`, `tools/nv/cmd/proofs.ts:15`
- [ ] It runs them two at a time through `spawnProof` and judges each reading with the overshoot
  bound. It prints the worst first, then the skips with their reasons. Exit 1 on a reading past the
  bound or a killed program. `tools/nv/proofs/run.ts:888`
- [ ] `USAGE`, the flag list and the module doc name `--probe`. `tools/nv/cmd/proofs.ts:70`, `tools/nv/cmd/proofs.ts:445`
- [ ] The four tests the `probe-judge` check names. Run once by name: `bun test tools/nv/test/probe.test.ts`. `tools/nv/test/probe.test.ts:1`
- [ ] Run `bun nv proofs --probe` once. Each red reading goes to stage 4's findings table. Record the
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

- [ ] The findings table: filled in by stages 1 to 3.
- [ ] `bun nv proofs --probe` exits 0.

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Not started. The goal was written on 2026-10-31 from the user's question about `memory-ceiling-holes`.

### Next

Stage 1 and stage 2, in either order. Stage 1's first item: `crates/nvs-runtime/src/ctx/wiring.rs:162`.

### Backlog
