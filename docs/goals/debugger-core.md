---
looper: 1
title: "A debugged program stops at a statement and shows its tasks, frames and variables"
summary: "Under the debug compile switch every statement is a stop site and every local has a described slot, nvs_safepoint acts on DEBUG_BREAK, a stop pauses the request's whole task tree with its deadlines suspended, a Rust-driven session reads each task's frames and variables, breakpoints survive a recompile, every designed rule and gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-host/src/scheduler.rs]
stages:
  - id: 1
    title: "the debug compile switch: described slots for every local"
    status: done
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/ir.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-codegen/src/emit.rs, docs/rules/testing/a-debugged-program-is-compiled-with-a-debug-switch.md]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, testing/debug-probes]
  - id: 5
    title: "a stop site per statement, and a frame record per call"
    status: done
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-codegen/tests/probes.rs, crates/nvs-runtime/src/ctx/mod.rs, docs/rules/testing/a-debugged-program-is-compiled-with-a-debug-switch.md]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, testing/debug-probes]
  - id: 2
    title: a task stops at an armed site under a Rust controller
    status: open
    context: [crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-ir/src/ir.rs, data/gaps/nvs-ir/debug-break-is-cleared-and-otherwise.json, data/gaps/nvs-runtime/nvs-safepoint-clears-debug-break-and.json]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, testing/a-stop-pauses-the-whole-task-tree]
  - id: 6
    title: a stopped task's frames and variables are read
    status: open
    context: [crates/nvs-host/src/debug.rs, crates/nvs-runtime/src/ctx/frames.rs, crates/nvs-runtime/src/record.rs]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, errors/record-transformations, security/redaction-reaches-the-tools-own-renderings]
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
    context: [crates/nvs-cli/src/script.rs, crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/debug.rs, docs/rules/testing/a-debugged-program-is-compiled-with-a-debug-switch.md]
    extra:
      context:
        rules: [testing/a-debugged-program-is-compiled-with-a-debug-switch, config/an-edit-reaches-the-next-request-without-a-restart]
  - id: 7
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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
is cleared and ignored (`crates/nvs-runtime/src/ctx/safepoint.rs:732`,
`crates/nvs-runtime/src/ctx/safepoint.rs:871`), and two gap records say so. A local variable is an SSA
value with no home a debugger could read (`crates/nvs-ir/src/lower/mod.rs:136`), and no table says
which variables a frame has. The safepoint poll sits only at function entry and loop back edges
(`crates/nvs-codegen/src/emit.rs:1156`), so most lines could never hold a breakpoint. This goal
builds the stop, the frames and the variables, driven from Rust. `debugger-adapter` puts `nvs dap` and
VS Code on top of it.

## Standing decisions

**A debug compile mode.** The user decided this. Under `nvs dap`, the same baseline compiler runs with
one switch: every local lives in a described stack slot, and every statement is a stop site. It is one
compiler with a switch, like the probes, and not a second tier; `rule:testing/debug-probes` already says
so (`docs/rules/testing/debug-probes.md:25`). Deoptimisation stays with the optimising tier. A debug
session runs slower, and a normal run and production pay nothing.
`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch` holds it.

**The stop is the request's task tree.** The user decided this. Every task of the debugged request or
program pauses at its next stop site, and each task is a DAP thread. Other requests on the same core
keep running. The stopped request's CPU and wall deadlines are suspended while it is stopped.
`rule:testing/a-stop-pauses-the-whole-task-tree` holds it.

**A stop site is the statement number the probes already use.** The statement probe already passes its
number (`crates/nvs-codegen/src/emit.rs:1230`), and `Program::stmt_spans` maps it to a span
(`crates/nvs-ir/src/ir.rs:466`). The safepoint poll needs no site number of its own, because each frame
record keeps its current stop site.

**The call stack comes from frame records, never from unwinding.** Each function compiled with the
switch links a record to its task on entry and unlinks it on return. The walk is then the same on
Windows, Linux and macOS, and needs no unwind tables for JIT code. The record type lives in a new
`crates/nvs-runtime/src/ctx/frames.rs`, beside the task's context.

**A Rust controller is the stop's interface,** in a new `crates/nvs-host/src/debug.rs`. The host owns
one debug controller per session: set and clear breakpoints, pause, resume, and receive a stop with each
task's frames and variables. This goal's tests drive it from Rust over fixture programs. Stepping and
DAP are `debugger-adapter`'s, and its gate is that this file's controller, the slot table and the frame
records exist.

**Trade-offs, all together.** Request path: nothing without the switch, and the probes are unchanged.
Under the switch, a store per local per statement, a load and a branch per stop site, and a link and an
unlink per call. Memory under the switch: one frame record per call in flight and one slot per local,
per task. Usability: a breakpoint on any statement, with every variable's value. Simplicity: one switch
on the existing compiler, and no platform-specific frame walking.

**A stop site is the unit's armed byte, then the safepoint word.** The armed bytes are a
`nvs_runtime::SiteTable` the unit owns, one byte per program statement, and not a pointer in `Ctx`.
So a debug unit always has a valid table to read, and arming a site reaches every request running that
unit. An armed site calls `nvs_debug_site`, which runs the handler `Ctx::set_debug_stop` installed.
The runtime does not know the scheduler, so the host supplies that handler, and stage 2 makes it park
the task. The frame record holds the function's index in `Program::functions`, and the unit maps the
index to a name.

**Lanes.** None: the stages run strictly in the order listed, 1, 5, 2, 6, 3, 4. Stages 1 and 5 share
`crates/nvs-codegen/src/emit.rs`; 2 and 6 share `crates/nvs-host/src/debug.rs`; 3 shares
`crates/nvs-runtime/src/ctx/safepoint.rs` and `crates/nvs-host/src/scheduler.rs` with 2; and 4 resolves
breakpoints through the controller 2 builds.

## Stages

### 1. the debug compile switch: described slots for every local

**Does:** with the switch, the lowering records the variables in scope at each statement, and codegen
stores each into a described slot.

- [x] The lowering takes the switch and, at each statement marker, records each variable in scope with the IR value it holds (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-ir/src/lower/mod.rs:136`, `crates/nvs-ir/src/lower/mod.rs:735`
- [x] The switch reaches codegen, and a unit compiled with it carries a slot table per function: name, declared type, slot. `crates/nvs-codegen/src/lib.rs:909`
- [x] Codegen stores each recorded value into its stack slot at the statement. `crates/nvs-codegen/src/emit.rs:516`
- [x] Codegen tests: a unit compiled without the switch is unchanged instruction for instruction, and one compiled with it has a slot per local (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-codegen/tests/probes.rs:104`

### 5. a stop site per statement, and a frame record per call

**Does:** with the switch, each statement is a stop site, and each call links a frame record.

- [x] A stop site per statement that tests its armed byte and the safepoint word with one load and one branch each, beside the statement probe (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-codegen/src/emit.rs:1496`
- [x] The frame record: function, current stop site, slot base, and the link to the caller's record. `crates/nvs-runtime/src/ctx/frames.rs:1`
- [x] Each function links its frame record on entry and unlinks it on every return path, a throw included. `crates/nvs-codegen/src/emit.rs:335`, `crates/nvs-codegen/src/emit.rs:1545`
- [x] Codegen tests: a site per statement, and a record chain that matches the call depth after a throw unwinds two frames (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-codegen/tests/probes.rs:469`

### 2. a task stops at an armed site under a Rust controller

**Does:** an armed site or `DEBUG_BREAK` parks the task, and the controller sets breakpoints, pauses and
resumes.

- [ ] The controller: breakpoints by file and line resolved to sites through `stmt_spans`, pause, resume, and a stop event (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-host/src/debug.rs:1`, `crates/nvs-ir/src/ir.rs:466`
- [ ] `nvs_safepoint` and the stop site act on `DEBUG_BREAK` under the switch and park the task on its own stack, and without the switch nothing raises it. `crates/nvs-runtime/src/ctx/safepoint.rs:744`, `crates/nvs-runtime/src/ctx/safepoint.rs:871`, `crates/nvs-host/src/scheduler.rs:1402`
- [ ] The test that `DEBUG_BREAK` is cleared is replaced by one that it stops a task and a resume continues it. `crates/nvs-runtime/src/ctx/safepoint.rs:1204`
- [ ] Both gap records deleted, and every comment that cites them rewritten. `data/gaps/nvs-ir/debug-break-is-cleared-and-otherwise.json:1`, `data/gaps/nvs-runtime/nvs-safepoint-clears-debug-break-and.json:1`

### 6. a stopped task's frames and variables are read

**Does:** the controller reads a stopped task's frames from its records and each variable's value.

**Split:** once the frame and variable reading lands, one writing subagent writes the three fixture
tests (a method, a closure and a generator). The session reviews them and keeps the redaction test.

- [ ] A stopped task's frames come from its records, with each frame's file and line (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-host/src/debug.rs:1`, `crates/nvs-runtime/src/ctx/frames.rs:1`
- [ ] Each variable renders as the bounded tree a dump uses, with a `secret` value redacted (`rule:security/redaction-reaches-the-tools-own-renderings`, `rule:errors/record-transformations`). `crates/nvs-runtime/src/record.rs:76`
- [ ] A Rust test that a `secret` local renders redacted. `crates/nvs-host/src/debug.rs:1`
- [ ] Rust tests over fixtures: a breakpoint stops at its line with the right variable values, in a method, a closure and a generator. `crates/nvs-host/src/debug.rs:1`

### 3. a stop pauses the whole task tree, and its deadlines wait

**Does:** every task of the stopped request parks, on every core, and the request's limits do not run
while it is stopped.

**Split:** once the tree stop and the deadline change land, one writing subagent writes the three tests
in the last item. The session reviews them.

- [ ] A stop raises `DEBUG_BREAK` on the tree's shared safepoint word, so every task parks at its next stop site and the controller reports each task as a thread (`rule:testing/a-stop-pauses-the-whole-task-tree`). `crates/nvs-runtime/src/ctx/safepoint.rs:574`, `crates/nvs-host/src/scheduler.rs:312`
- [ ] A `spawn worker` child on another core stops and resumes with its tree (`rule:concurrency/on-worker-runs-the-child-on-another-core`). `crates/nvs-host/src/placed.rs:132`
- [ ] The time stopped is added to the wall deadline and is not charged as CPU time (`rule:errors/on-limit`). `crates/nvs-runtime/src/ctx/limits.rs:701`, `crates/nvs-runtime/src/ctx/limits.rs:757`, `crates/nvs-runtime/src/ctx/safepoint.rs:684`
- [ ] The watchdog does not count the stopped tree (`rule:http-server/a-wedged-core-is-detected-by-its-deadline`). `crates/nvs-host/src/watchdog.rs:440`, `crates/nvs-host/src/watchdog.rs:1241`
- [ ] Rust tests: a stop past the wall limit does not end the request, a second request on the same core is answered while the first is stopped, and a resumed tree finishes with the right output (`rule:testing/a-stop-pauses-the-whole-task-tree`). `crates/nvs-host/src/debug.rs:1`

### 4. breakpoints survive a recompile under a development `nvs serve`

**Does:** after an edit, each breakpoint resolves again against the new unit.

- [ ] A recompile resolves every breakpoint again by file and line, and a breakpoint on a line that no longer has a statement is reported as unresolved (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-cli/src/script.rs:714`, `crates/nvs-cli/src/script.rs:1500`
- [ ] A request already running finishes on the unit it started on, with that unit's sites (`rule:config/an-edit-reaches-the-next-request-without-a-restart`). `crates/nvs-host/src/debug.rs:1`
- [ ] A Rust test that edits a fixture under a served program and stops at the same breakpoint in the new unit. `crates/nvs-cli/src/script.rs:1500`
- [ ] Both rules `shipped`, and `bun nv owners --closes debugger-core` passes (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`, `rule:testing/a-stop-pauses-the-whole-task-tree`). `data/rules/testing/a-debugged-program-is-compiled-with-a-debug-switch.json:3`, `data/rules/testing/a-stop-pauses-the-whole-task-tree.json:3`

### 7. the goal is verified once, and every failure is fixed

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

Stages 1 and 5 are done. The switch is `nvs_ir::lower::with_debug_slots(|| lower...)`. Under it,
`Function::debug_slots` is `Some(table)`, empty for a function with no variables, and `None` without
the switch. Each `StmtMarker` is followed by an `InstKind::DebugLocals` whose error edge is
`landing_block(env)`. Codegen emits it as the slot stores and then `Emitter::emit_stop_site`: write the
site into the frame record, `uload8` of the site's armed byte then a branch to `nvs_debug_site`, then a
load of the safepoint word and a branch to `nvs_safepoint`. Both statuses go to the error edge.

The frame record is `nvs_runtime::FrameRecord` (`crates/nvs-runtime/src/ctx/frames.rs`): caller,
slot base (null without variables), function index, current site. `link_frame_record` links it in the
ABI entry block through `Ctx::frames` at `DEBUG_FRAMES_OFFSET`, and `Emitter::emit_return` unlinks it
at every `return_` the emitter makes. `Ctx::frames()` walks the chain innermost first.
`Unit::debug_function(index)` names a record's function. The armed bytes are the unit's
`nvs_runtime::SiteTable` (`Unit::debug_sites()`), one per program statement, published to the JIT as
`DEBUG_SITES_SYMBOL` and kept alive by the code pages. An armed site runs the context's
`Ctx::set_debug_stop` handler, and runs on when none is set. `DEBUG_BREAK` is still cleared and
ignored. Without the switch no instruction changes, so no test is expected to go red.

### Next

**Stage 2: a task stops at an armed site under a Rust controller**, file set
`crates/nvs-host/src/debug.rs` (new), `crates/nvs-runtime/src/ctx/safepoint.rs`,
`crates/nvs-runtime/src/ctx/frames.rs`, `crates/nvs-host/src/scheduler.rs`. No gate.

- [ ] The controller sets a breakpoint by file and line: the sites on that line from `Program::stmt_spans`, armed in `Unit::debug_sites()`. `crates/nvs-codegen/src/lib.rs:649`, `crates/nvs-ir/src/ir.rs:48`
- [ ] The stop handler the host installs with `Ctx::set_debug_stop` parks the task on its own stack through the scheduler's suspend, and a resume continues it. `crates/nvs-runtime/src/ctx/frames.rs:126`, `crates/nvs-runtime/src/ctx/frames.rs:201`
- [ ] `safepoint_slow_path` acts on `DEBUG_BREAK` by running the same handler, and the test that it is cleared is replaced. `crates/nvs-runtime/src/ctx/safepoint.rs:932`, `crates/nvs-runtime/src/ctx/safepoint.rs:1470`
- [ ] The debug compile turns the artifact cache off. `crates/nvs-codegen/src/lib.rs:963`
- [ ] Both gap records deleted, and every comment that cites them rewritten. `data/gaps/nvs-runtime/nvs-safepoint-clears-debug-break-and.json:1`

### Backlog

- A debug unit must never be stored in or loaded from the artifact cache, whose key does not cover the switch. `Unit::into_unit` gives a cached unit no slot table and no site table, and the object backend cannot resolve `DEBUG_SITES_SYMBOL`. Stage 2 or 4, whichever first compiles a program under the switch, turns the cache off for it, as `compile_checked_types` does. `crates/nvs-codegen/src/lib.rs:963`
- The synthesized functions `nvs-ir` builds itself (`lower/call.rs`, `lower/exception.rs`) have `debug_slots: None`, so they get no frame record and do not show in the call stack. That is the intended view: they are not the user's code.
- An `inout` by-reference variable has no slot, so the debugger cannot show it yet. Stage 6 can read it through its staged slot. `crates/nvs-ir/src/lower/mod.rs:2602`
- Stepping, exception breakpoints and `nvs dap` itself are `debugger-adapter`'s (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
- The value a function returned, shown after a step out, needs the frame record to keep it (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
