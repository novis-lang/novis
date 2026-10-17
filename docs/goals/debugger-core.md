---
looper: 1
title: "A debugged program stops at a statement and shows its tasks, frames and variables"
summary: "Under the debug compile switch every statement is a stop site and every local has a described slot, nvs_safepoint acts on DEBUG_BREAK, a stop pauses the request's whole task tree with its deadlines suspended, a Rust-driven session reads each task's frames and variables, breakpoints survive a recompile, every designed rule and gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-host/src/scheduler.rs]
stages:
  - id: 1
    title: "the debug compile switch: described slots and a stop site per statement"
    status: open
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/print.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-codegen/src/emit.rs, docs/rules/testing/a-debugged-program-is-compiled-with-a-debug-switch.md]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, testing/debug-probes]
  - id: 2
    title: a task stops at an armed site, and its frames and variables are read
    status: open
    context: [crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-runtime/src/record.rs, crates/nvs-host/src/scheduler.rs, data/gaps/nvs-ir/debug-break-is-cleared-and-otherwise.json, data/gaps/nvs-runtime/nvs-safepoint-clears-debug-break-and.json]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, testing/a-stop-pauses-the-whole-task-tree, errors/record-transformations, security/redaction-reaches-the-tools-own-renderings]
  - id: 3
    title: a stop pauses the whole task tree, and its deadlines wait
    status: open
    context: [crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/placed.rs, crates/nvs-host/src/watchdog.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/ctx/limits.rs, docs/rules/testing/a-stop-pauses-the-whole-task-tree.md]
    extra:
      context:
        rules: [testing/a-stop-pauses-the-whole-task-tree, errors/on-limit, http-server/a-wedged-core-is-detected-by-its-deadline, concurrency/on-worker-runs-the-child-on-another-core]
  - id: 4
    title: breakpoints survive a recompile under a development `nvs serve`
    status: open
    context: [crates/nvs-cli/src/script.rs, crates/nvs-host/src/scheduler.rs, docs/rules/testing/a-debugged-program-is-compiled-with-a-debug-switch.md]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, config/an-edit-reaches-the-next-request-without-a-restart]
checks:
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, debugger-core]
    reach: true
    floor: false
---

# A debugged program stops at a statement and shows its tasks, frames and variables

## Why

Nothing in Novis can stop a running program. The safepoint flag a debugger would raise, `DEBUG_BREAK`,
is cleared and ignored (`crates/nvs-runtime/src/ctx/safepoint.rs:732-733`,
`crates/nvs-runtime/src/ctx/safepoint.rs:870`), and two gap records say so. A local variable is an SSA
value with no home a debugger could read (`crates/nvs-ir/src/lower/mod.rs:136`), and no table says
which variables a frame has. The safepoint poll sits only at function entry and loop back edges
(`crates/nvs-codegen/src/emit.rs:1144-1169`), so most lines could never hold a breakpoint. This goal
builds the stop, the frames and the variables, driven from Rust. `debugger-adapter` puts `nvs dap` and
VS Code on top of it.

## Standing decisions

**A debug compile mode.** The user decided this. Under `nvs dap`, the same baseline compiler runs with
one switch: every local lives in a described stack slot, and every statement is a stop site. It is one
compiler with a switch, like the probes, and not a second tier, so `rule:testing/debug-probes` is
restated to say so. Deoptimisation stays with the optimising tier. A debug session runs slower, and a
normal run and production pay nothing. `rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`
holds it.

**The stop is the request's task tree.** The user decided this. Every task of the debugged request or
program pauses at its next stop site, and each task is a DAP thread. Other requests on the same core
keep running. The stopped request's CPU and wall deadlines are suspended while it is stopped.
`rule:testing/a-stop-pauses-the-whole-task-tree` holds it.

**A stop site is the statement number the probes already use.** The planning session's call. The
statement probe already passes its number (`crates/nvs-codegen/src/emit.rs:1231-1233`), and
`Program::stmt_spans` maps it to a span (`crates/nvs-ir/src/print.rs:63`). The safepoint poll needs no
site number of its own, because each frame record keeps its current stop site.

**The call stack comes from frame records, never from unwinding.** The planning session's call. Each
function compiled with the switch links a record to its task on entry and unlinks it on return. The walk
is then the same on Windows, Linux and macOS, and needs no unwind tables for JIT code.

**A Rust controller is the stop's interface.** The planning session's call. The host owns one debug
controller per session: set and clear breakpoints, pause, resume, and receive a stop with each task's
frames and variables. This goal's tests drive it from Rust over fixture programs. Stepping and DAP are
`debugger-adapter`'s.

**Trade-offs, all together.** Request path: nothing without the switch, and the probes are unchanged.
Under the switch, a store per local per statement, a load and a branch per stop site, and a link and an
unlink per call. Memory under the switch: one frame record per call in flight and one slot per local,
per task. Usability: a breakpoint on any statement, with every variable's value. Simplicity: one switch
on the existing compiler, and no platform-specific frame walking.

## Stages

### 1. the debug compile switch: described slots and a stop site per statement

**Does:** with the switch, the lowering records the variables in scope at each statement, and codegen
stores each into a described slot and emits a stop site.

- [ ] The lowering takes the switch and, at each statement marker, records each variable in scope with the IR value it holds (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-ir/src/lower/mod.rs:136`, `crates/nvs-ir/src/lower/mod.rs:733`
- [ ] Codegen stores each recorded value into its stack slot at the statement, and writes a slot table per function: name, declared type, slot. `crates/nvs-codegen/src/lib.rs:810`, `crates/nvs-codegen/src/emit.rs:516`
- [ ] A stop site per statement that tests its armed byte and the safepoint word with one load and one branch each, beside the statement probe. `crates/nvs-codegen/src/emit.rs:1218-1238`
- [ ] Each function links its frame record on entry and unlinks it on every return path, a throw included.
- [ ] Codegen tests: a unit compiled without the switch is unchanged instruction for instruction, and one compiled with it has a slot per local and a site per statement. `crates/nvs-codegen/tests/probes.rs`

### 2. a task stops at an armed site, and its frames and variables are read

**Does:** an armed site or `DEBUG_BREAK` parks the task, and the controller reads its frames and
values.

- [ ] The controller: breakpoints by file and line resolved to sites through `stmt_spans`, pause, resume, and a stop event. `crates/nvs-ir/src/print.rs:63`
- [ ] `nvs_safepoint` and the stop site act on `DEBUG_BREAK` under the switch and park the task on its own stack, and without the switch nothing raises it. `crates/nvs-runtime/src/ctx/safepoint.rs:744`, `crates/nvs-runtime/src/ctx/safepoint.rs:870`, `crates/nvs-host/src/scheduler.rs:1342`
- [ ] A stopped task's frames come from its records, and each variable renders as the bounded tree a dump uses, with a `secret` value redacted (`rule:security/redaction-reaches-the-tools-own-renderings`). `crates/nvs-runtime/src/record.rs:76`
- [ ] The test that `DEBUG_BREAK` is cleared is replaced by one that it stops, and both gap records are deleted. `crates/nvs-runtime/src/ctx/safepoint.rs:1204`
- [ ] Rust tests over a fixture: a breakpoint stops at its line with the right variable values, in a method, a closure and a generator.

### 3. a stop pauses the whole task tree, and its deadlines wait

**Does:** every task of the stopped request parks, on every core, and the request's limits do not run
while it is stopped.

- [ ] A stop raises `DEBUG_BREAK` on the tree's shared safepoint word, so every task parks at its next stop site and the controller reports each task as a thread (`rule:testing/a-stop-pauses-the-whole-task-tree`). `crates/nvs-runtime/src/ctx/safepoint.rs:574`, `crates/nvs-host/src/scheduler.rs:306`
- [ ] A `spawn worker` child on another core stops and resumes with its tree. `crates/nvs-host/src/placed.rs:1-8`
- [ ] The time stopped is added to the wall deadline and is not charged as CPU time, and the watchdog does not count the stopped tree. `crates/nvs-runtime/src/ctx/limits.rs:701-790`, `crates/nvs-runtime/src/ctx/safepoint.rs:684`, `crates/nvs-host/src/watchdog.rs:1-6`
- [ ] Tests: a stop past the wall limit does not end the request, a second request on the same core is answered while the first is stopped, and a resumed tree finishes with the right output.

### 4. breakpoints survive a recompile under a development `nvs serve`

**Does:** after an edit, each breakpoint resolves again against the new unit.

- [ ] A recompile resolves every breakpoint again by file and line, and a breakpoint on a line that no longer has a statement is reported as unresolved. `crates/nvs-cli/src/script.rs:715`, `crates/nvs-cli/src/script.rs:1501`
- [ ] A request already running finishes on the unit it started on, with that unit's sites (`rule:config/an-edit-reaches-the-next-request-without-a-restart`).
- [ ] A test that edits a fixture under a served program and stops at the same breakpoint in the new unit. Both rules `shipped`, and `bun nv owners --closes debugger-core` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written. `crates/nvs-runtime/src/ctx/safepoint.rs` and `crates/nvs-runtime/src/ctx/limits.rs` were being
edited by another session at the time, so find their anchors by name if a line has moved.

### Next

Stage 1, the debug compile switch, on `crates/nvs-ir/src/lower/mod.rs` and
`crates/nvs-codegen/src/emit.rs`:
- record each variable in scope at a statement (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-ir/src/lower/mod.rs:136`
- the slot table and the stores. `crates/nvs-codegen/src/emit.rs:516`
- the stop site beside the statement probe. `crates/nvs-codegen/src/emit.rs:1218`

### Backlog

- Stepping, exception breakpoints and `nvs dap` itself are `debugger-adapter`'s (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
- The value a function returned, shown after a step out, needs the frame record to keep it (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
