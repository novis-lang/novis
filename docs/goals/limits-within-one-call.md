---
looper: 1
title: "a native call stops at the wall-time, CPU and stack limits as it does at the memory ceiling"
summary: "every wait a member makes ends at the request's deadline, native recursion checks the stack before it goes deeper, and the overshoot scan judges CPU and wall-time readings"
context: [crates/nvs-runtime/src/ctx/limits.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-host/src/watchdog.rs, tools/nv/proofs/run.ts]
stages:
  - id: 1
    title: "a survey lists every wait, every native recursion and every member whose cost its caller picks"
    status: open
  - id: 2
    title: "native recursion checks the stack before it goes deeper"
    status: open
    context: [crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-host/src/stack.rs]
  - id: 3
    title: "every wait a member makes ends at the request's deadline"
    status: open
  - id: 4
    title: "a CPU or wall-time FATAL names what the request used, and the overshoot scan judges it"
    status: open
    context: [crates/nvs-runtime/src/abi.rs, tools/nv/cmd/proofs.ts]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: deep-native-recursion
    stage: 2
    name: native recursion past the stack ends the request with a FATAL and the process goes on
    run: [cargo, test, --lib, native_recursion_past_the_stack]
    tests:
      - native_recursion_past_the_stack_ends_the_request_and_not_the_process
  - id: time-overshoot
    stage: 4
    name: every attack that ends at its CPU or wall-time limit stopped within its bound
    run: [bun, nv, proofs, --overshoot]
    timeout: 1800
    reach: true
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, limits-within-one-call]
    reach: true
    floor: false
---

# a native call stops at the wall-time, CPU and stack limits as it does at the memory ceiling

## Why

A request's limits are noticed anywhere and acted on only at a poll: before and after each `Core`
member, and at the JIT's safepoints. `memory-ceiling-holes` closed that gap for memory. The other
limits have the same shape, and nobody has looked at them the same way:

- **Wall time.** The deadline is checked at a poll. `Core\Time::sleep` is the one member that ends a
  wait at the deadline (`crates/nvs-runtime/src/ctx/limits.rs:987`). Any other blocking wait inside a
  member runs to its own timeout.
- **CPU time.** `nvs_host::watchdog` samples the thread's CPU clock and raises `CPU_LIMIT`
  (`crates/nvs-host/src/watchdog.rs`). Work proportional to its input is bounded by the memory ceiling.
  Work that grows faster than its input, or whose cost the caller picks, is not.
- **Stack.** Compiled code checks the stack at function entry (`crates/nvs-runtime/src/ctx/safepoint.rs:1216`,
  `STACK_CEILING` at `crates/nvs-runtime/src/ctx/mod.rs:1905`). Native Rust recursion has depth caps
  written per site (`json.rs`, `graph.rs`, `signature.rs` and others). A stack overflow in native code
  aborts the process, which ends every request. This is the most serious of the three.

The user's call (2026-10-31): the work starts with a survey, and a later stage does only what the
survey shows is needed.

## Standing decisions

**The survey sets the scope.** Stage 1 writes the item lists of stages 2, 3 and 4. A stage whose
survey list is empty is closed with one line saying so. Nothing is changed in a site the survey did not
list.

**One stack guard, from the existing pair.** Native recursion asks one helper in `nvs-runtime` whether
the stack has room before it goes deeper. The helper compares the stack pointer with the floor the
context already arms (`stack_floor`, `crates/nvs-runtime/src/ctx/safepoint.rs:27`). A refusal is the
same `FATAL` the compiled code's check gives. No new dependency: `stacker` and `psm` were considered,
and the existing pair already knows each stack's bounds, task stacks included
(`crates/nvs-host/src/stack.rs`). A per-site depth cap that gives a catchable error at a sensible depth,
such as JSON's nesting limit, stays: it is a format limit, not a stack guard. Cost: one load and one
compare per recursive step.

**A wait takes the deadline it is given, never a new setting.** A blocking wait inside a member reads
`Ctx::wall_time_left` and waits no longer than the shorter of that and its own timeout, as
`Core\Time::sleep` does. Reaching the deadline gives `wall_time_breach`'s `FATAL`. No lint is added in
this goal: the survey's list is the set, and the decision on a lint waits for its size.

**CPU work whose cost the caller picks is checked before it starts.** Each such member the survey
lists checks its cost against the CPU time left before the work, the way `affordable` checks a count
against memory. Where the cost cannot be known first, the member polls every so many steps, and its
doc comment says how many. The regex backtracking setting (`[regex] backtracking`) already covers
backtracking patterns and is not changed.

**A time limit's overshoot bound is the CPU or wall-time limit plus 100 ms.** The `FATAL` names the
time the request used, the way a memory `FATAL` names its peak. The overshoot judge reads it, and a
reading past the bound is red. The 100 ms is the watchdog's sampling interval plus scheduling noise.
Stage 4 measures the legitimate worst over the existing attacks first, and sets the slack to twice
that if it is larger. The bound is never raised to make an attack pass.

**Lanes.** Stage 1 runs first. Stages 2, 3 and 4 touch disjoint files once the survey is in, and each
may run as a side goal.

**Trade-offs.**
- Security: a request cannot crash the process through native recursion, and cannot hold a core past
  its time limits inside one call.
- Latency: one compare per recursive step, and one clock read per wait.
- Memory: nothing is added.
- Simplicity: one stack helper replaces the reasoning each site does today.

## Stages

### 1. a survey lists every wait, every native recursion and every member whose cost its caller picks

**Split:** three read-only `reader` subagents, side by side, over `crates/nvs-stdlib`,
`crates/nvs-db`, `crates/nvs-ldap`, `crates/nvs-host` and `crates/nvs-runtime`. Each returns
`file.rs:NN`, the member that reaches the site and one line each.

- [ ] Every blocking wait a member can reach: a socket read or write, a database reply, a child
  process, a lock or a channel. For each: whether it reads the request's deadline today. Written as
  stage 3's items.
- [ ] Every native recursion a request's input can drive: a parser, a walk, an encoder, a matcher. For
  each: its depth cap if it has one, and the depth at which the stack would end first. Written as
  stage 2's items.
- [ ] Every member whose CPU cost grows faster than its input or is picked by an argument. Examples to
  check: an edit distance, a password hash's cost, decompression, sorting with a callback. Written as
  stage 4's items.
- [ ] Whether the output limit is written through one path. If it is not, each other path is a stage 4
  item.

### 2. native recursion checks the stack before it goes deeper

- [ ] The stack helper in `nvs-runtime`, by the standing decision. `crates/nvs-runtime/src/ctx/safepoint.rs:1216`
- [ ] The test `native_recursion_past_the_stack_ends_the_request_and_not_the_process`, which drives
  one listed recursion past the stack. Run once by name.
- [ ] Each recursion stage 1 listed asks the helper. Filled in by stage 1.
- [ ] An attack step per listed recursion that a `.nvs` file can reach, ending at `FATAL`.

### 3. every wait a member makes ends at the request's deadline

- [ ] Each wait stage 1 listed reads `wall_time_left`, by the standing decision. Filled in by stage 1.
- [ ] An attack step per listed wait a `.nvs` file can reach, ending at the wall-time `FATAL`. A wait
  only a remote peer can hold open is a Rust test with a scripted peer.

### 4. a CPU or wall-time FATAL names what the request used, and the overshoot scan judges it

- [ ] The CPU and wall-time `FATAL`s name the time the request used and its limit. `crates/nvs-runtime/src/abi.rs:381`,
  `crates/nvs-runtime/src/ctx/safepoint.rs:1046`, `crates/nvs-runtime/src/ctx/limits.rs:997`
- [ ] `judgeHostile` reads that time and applies the time bound. `bun nv proofs --overshoot` includes
  attacks that end at a time limit. `tools/nv/proofs/run.ts:888`, `tools/nv/proofs/run.ts:976`
- [ ] Measure the legitimate worst over the existing time attacks, and set the slack by the standing
  decision.
- [ ] Each member stage 1 listed checks its cost first or polls. Filled in by stage 1.
- [ ] `bun nv proofs --overshoot` exits 0.

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Not started. The goal was written on 2026-10-31, beside `memory-overshoot-net`.

### Next

Stage 1, the survey. Its three lanes are listed under its `**Split:**`.

### Backlog
