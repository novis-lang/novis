---
looper: 1
title: "a cancelled task's teardown runs cleanly in every build profile"
summary: "a parked task cancelled by the scheduler, by another task or by the scheduler's drop unwinds without a crash on Windows in the debug, proof and release profiles, and nvs run cancels a queue worker still connecting, so a Windows run with an absent database ends without the 2 s wait"
context: [crates/nvs-host/src/scheduler.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-cli/src/worker.rs, docs/rules/concurrency/cancellation-runs-no-user-code.md, Cargo.toml]
stages:
  - id: 1
    title: "a baseline is recorded, and a test of every teardown path crashes on the proof profile"
    status: done
    context: [crates/nvs-host/src/scheduler.rs, crates/nvs-host/tests/deferred.rs, crates/nvs-cli/src/worker.rs, tools/nv/proofs/run.ts]
  - id: 2
    title: "the crash's cause is named and fixed, and CI runs the teardown tests on the proof profile"
    status: open
    context: [crates/nvs-host/src/scheduler.rs, crates/nvs-host/src/tsan.rs, crates/nvs-runtime/src/abi.rs, crates/nvs-cli/tests/one_binary.rs, .github/workflows/ci.yml]
  - id: 3
    title: "nvs run's stop cancels a queue worker still connecting, and every proof program is green on the proof binary"
    status: done
    context: [crates/nvs-cli/src/worker.rs, crates/nvs-cli/tests/check_command.rs, docs/rules/concurrency.md, docs/rules/concurrency/one-process-serves-requests-schedules-and-jobs.md]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
    context: [tools/nv/cmd/ci.ts, .github/workflows/ci.yml]
checks:
  - id: teardown-debug
    stage: 1
    name: every teardown path of a parked task unwinds and drops what the task held, in the debug profile
    run: [cargo, test, --test, forced_unwind]
    timeout: 1800
    tests:
      - a_parked_task_cancelled_by_the_scheduler_unwinds_and_drops_what_it_held
      - a_parked_task_cancelled_from_inside_another_task_unwinds_and_drops_what_it_held
      - a_scheduler_dropped_with_a_parked_task_unwinds_it_and_drops_what_it_held
      - a_task_cancelled_before_its_first_turn_drops_its_body_unrun
      - a_thousand_parked_tasks_cancelled_in_one_turn_each_drop_once
  - id: teardown-proof
    stage: 2
    name: every teardown path of a parked task unwinds and drops what the task held, in the proof profile
    run: [cargo, test, --profile, proof, --test, forced_unwind]
    timeout: 3600
    tests:
      - a_parked_task_cancelled_by_the_scheduler_unwinds_and_drops_what_it_held
      - a_parked_task_cancelled_from_inside_another_task_unwinds_and_drops_what_it_held
      - a_scheduler_dropped_with_a_parked_task_unwinds_it_and_drops_what_it_held
      - a_task_cancelled_before_its_first_turn_drops_its_body_unrun
      - a_thousand_parked_tasks_cancelled_in_one_turn_each_drop_once
  - id: server-retire-proof
    stage: 2
    name: a proof-profile server stopped with a request still parked exits cleanly
    run: [cargo, test, --profile, proof, --test, one_binary, --, a_server_stopped_with_a_request_still_parked_exits_cleanly]
    timeout: 3600
    tests:
      - a_server_stopped_with_a_request_still_parked_exits_cleanly
  - id: queue-worker-cancel-debug
    stage: 3
    name: nvs run ends without waiting for a queue worker still connecting, in the debug profile
    run: [cargo, test, --test, check_command]
    timeout: 1800
    tests:
      - run_ends_without_waiting_for_a_queue_worker_still_connecting
      - run_count_starts_no_queue_worker
  - id: queue-worker-cancel-proof
    stage: 3
    name: nvs run ends without waiting for a queue worker still connecting, on the proof binary
    run: [cargo, test, --profile, proof, --test, check_command, --, run_ends_without_waiting_for_a_queue_worker_still_connecting]
    timeout: 3600
    tests:
      - run_ends_without_waiting_for_a_queue_worker_still_connecting
---

# a cancelled task's teardown runs cleanly in every build profile

## Why

Cancelling a parked task tears its stack down by a forced unwind (`Task::force_unwind`,
`crates/nvs-host/src/scheduler.rs:706`). On Windows, in the optimized profiles (`proof`, `release`),
that unwind crashes the process with an access violation (`0xC0000005`, a proof's `exit 5`). The debug
profile unwinds cleanly, so every crate test is green while the shipped binary crashes.

What is known:

- The unwind runs every cleanup up to the task's entry closure in `Scheduler::start`
  (`crates/nvs-host/src/scheduler.rs:931`, `let entry = move |yielder`), and crashes once it leaves
  that closure's first cleanup, on its way into `corosensei`'s stack-switch frame.
- It crashed with the context kept out of the unwind too, so the context's `Drop` is not the cause.
- `corosensei` is at 0.3.4 (`Cargo.lock`). The Windows unwind across its stack switch is its code.
- It was found through 63a6861d4, which made `nvs run`'s stop cancel a queue worker still connecting.
  Every proof program run from the repository root crashed after its output (the root `nvs.toml` sets
  `[queue] workers = 1`, and no database answers), 1153 proofs went red, and cb931f1ef reverted it.

Two things wait on the fix:

- **Servers.** `Drop for Scheduler` (`crates/nvs-host/src/scheduler.rs:806`) and `tear_down`
  (`crates/nvs-host/src/scheduler.rs:1239`) take the same path for every unwindable task. A server
  worker retiring with such a task parked may crash the same way. That is a user-facing crash, and it
  is why the fix is in the scheduler and not in the queue worker.
- **CI time.** Without the cancel, a Windows `nvs run` against a database that is down waits out the
  worker's 2 s `CONNECT_DEADLINE` (`crates/nvs-cli/src/worker.rs:152`). That is about 1,300 proof
  programs at +2 s each, and it makes `proofs / windows-x86_64` the longest CI job (56 min). With the
  cancel, a parked program measured 2050 ms before and 155 ms after on this machine.

Everything here reproduces on a Windows machine. CI adds macOS and Linux aarch64, which use another
unwinder and never showed the crash.

## Standing decisions

**This goal adds no bug while it fixes one.** Each point below is an item in a stage, not advice:

1. **A baseline before any change.** Stage 1 records what is red before the goal touches code: the
   floor's red list from `bun nv debt`, and the proof-profile runs of `deferred`, `limits` and
   `placed_path` in `crates/nvs-host/tests/`. A red that appears later and is not on that list is this
   goal's, and is fixed before the stage is ticked.
2. **Red first.** Every new test is seen crashing (or failing) on the proof profile before the fix,
   and the handoff records the exit code of each. A test that never failed proves nothing about the
   fix.
3. **The fix follows the named cause, and only that.** Stage 2 names the cause in writing before it
   changes code. The diff touches the files the cause names. Cancellation keeps its semantics:
   `rule:concurrency/cancellation-runs-no-user-code` holds, a task is still unwound where it stands,
   every native `Drop` on the task's stack still runs exactly once, and the stack still goes back to
   the pool.
4. **Every teardown path, every profile.** The tests cover a cancel by the scheduler, a cancel from
   inside another task (`cancel_task`, `crates/nvs-host/src/scheduler.rs:1570`), the scheduler's
   drop with a task parked, a task cancelled before its first turn, and a thousand tasks cancelled in
   one turn (a crash that depends on frame layout can be intermittent). Each runs in the debug and
   the proof profile. A proof-profile server test covers a worker retiring with a request parked. A
   change to a path, socket or process also runs in WSL with `bun nv linux-test`.
5. **The behaviour of the queue worker is pinned before the cancel goes back in.** A worker that
   holds a claimed job still finishes or writes the job back at a stop, a drain still cancels nothing,
   and `nvs run --count` still starts no worker. Each has a test before stage 3 changes
   `crates/nvs-cli/src/worker.rs`.
6. **The whole proof corpus on the proof binary.** Stage 3 runs `bun nv debt` with the cancel back in,
   and the floor is no redder than the stage-1 baseline.
7. **CI keeps it fixed.** The proofs job runs the teardown tests on the proof profile on every OS, so
   a later change that brings the crash back is red in CI, not in a user's server.
8. **The gate pushes and watches CI to green**, macOS and aarch64 included, before `DONE`.

**These are not fixes, and no session lands one.** Each hides the crash or moves it:

- Skipping the forced unwind on Windows and forgetting the task. The task's `Drop`s never run, so a
  half-open socket stays open and the stack is never returned: a leak that grows with requests served.
- Catching the access violation (a vectored exception handler, an SEH filter) and carrying on.
- Lowering `opt-level`, turning off LTO or changing `panic` for a crate or a profile. Each is allowed
  as a diagnostic step in stage 2 and reverted after. If the cause turns out to be a compiler or
  `corosensei` bug that one function's inlining exposes, an `#[inline(never)]` on that one function
  is a fix, with a comment that names the cause and the test that proves it.
- Cancelling the queue worker some other way (a cooperative flag in the connect loop) and leaving the
  scheduler's crash in place. The server path would still crash.

**If the cause is in `corosensei`:** first a newer `corosensei` 0.3 release, if its changelog or code
fixes it. Otherwise a `[patch.crates-io]` entry with the smallest change, its source under
`vendor/corosensei/`, and a comment in `Cargo.toml` naming the cause. Filing the issue upstream
publishes outside the repository: the handoff says what to file, and the user files it.

**No debugger is installed on this machine.** Stage 2 narrows the crash with the cheap steps its items
list. Installing one is not a step the loop takes.

**Where the answer is written.** The cause and the fix go in `Task::force_unwind`'s doc comment
(`crates/nvs-host/src/scheduler.rs:697`), the home of how a task is unwound. The CI step's reason goes
in its comment in `.github/workflows/ci.yml`.

**CI step's home.** The proofs job already builds the proof profile (`.github/workflows/ci.yml:583`),
so it runs `cargo test --profile proof --test forced_unwind` after that build. Stage 2 measures what the
step adds on the Windows leg. If it adds more than 3 minutes, the five cases move into a `#[cfg(test)]`
module of `nvs-cli`'s lib, which the cost guards' `cargo test --release` step already compiles
(`.github/workflows/ci.yml:490`), and the check's `run` moves with them.

**The server test pins the stop's exit status, and it cannot go red on the crash.** Point 2 is not
met by it, and that is accepted. A stopped server's core turns until nothing is parked
(`crates/nvs-cli/src/serve.rs:1653`), and a task parked in a script is forgotten at the scheduler's
drop, not unwound (`crates/nvs-host/src/scheduler.rs:841`). So no stop of a server reaches
`Task::force_unwind`, and the test stayed green with `native_frame` inlined while `forced_unwind`
crashed with `0xC0000005`. A server path into the unwind comes only from a cancel of a task with no
script on its stack. Stage 3's queue-worker cancel is the first one, and if `nvs serve`'s drain uses it
too, stage 3 extends the server test to reach it and sees it red with the fix undone.

**Lanes.** None. Each stage builds on the one before, and stages 2 and 3 share the scheduler's tests.

**The floor's baseline is the sweep's `floor-debt` log, not a session's run.** The looper guard refuses
`bun nv debt` inside a session (`heavy-run`): looper pays it once at the goal end as the floor check
`floor-debt`. Stage 1 read `.looper/loops/main/checks/floor-debt.log` instead, and stage 3's "run
`bun nv debt`" item is met by `bun nv proofs --run` over the groups the cancel reaches (`bun nv affected`
names them) plus the goal end's `floor-debt` check. Running every proof in a session costs the loop an
hour for what the goal end runs anyway.

## Stages

### 1. a baseline is recorded, and a test of every teardown path crashes on the proof profile

**Does:** records the baseline, then writes the five teardown tests. They are green in debug. The
session runs each alone on the proof profile and records which crash. Nothing is fixed here.

- [x] Record the baseline in the handoff: `bun nv debt`'s red list, and
  `cargo test --profile proof --test deferred --test limits --test placed_path`.
  `crates/nvs-host/tests/deferred.rs:1`
- [x] Write `crates/nvs-host/tests/forced_unwind.rs` with the five tests the `teardown-debug` check
  names. Each parked task waits the way the queue worker's connect does
  (`crates/nvs-cli/src/worker.rs:1920`): a native Rust body that suspends with `nvs_host::suspend`
  and holds a value whose `Drop` counts. Each test asserts the count is exactly one per task and that
  the scheduler runs on afterwards. No task parks under an `extern "C"` frame
  (`docs/agent/playbook/writing-a-test-case/a-nativebody-slot-or-task-that-parks-in-nvshost-sleep-aborts.md`
  says why). `crates/nvs-host/tests/forced_unwind.rs:1`
- [x] Run `cargo test --test forced_unwind`: green.
- [x] Run each test alone on the proof profile
  (`cargo test --profile proof --test forced_unwind -- <name> --exact`) and record each exit code in
  the handoff.
- [x] If none crashes, the test lacks what the queue worker has. Re-apply 63a6861d4's change to
  `crates/nvs-cli/src/worker.rs` uncommitted, build the proof binary, run one example from the
  repository root with its output in a file, and confirm the crash. Then move the test closer to
  the worker's shape until it crashes, and remove the uncommitted change.
  `crates/nvs-cli/src/worker.rs:247`
- [x] Commit the test file. It is green in debug, and nothing runs it optimized yet.

### 2. the crash's cause is named and fixed, and CI runs the teardown tests on the proof profile

**Does:** finds the cause with the stage-1 tests, writes it down, fixes it in the scheduler (or in
`corosensei`, as § *Standing decisions* says), and adds the CI step.

**Split:** reading `corosensei` 0.3.4's Windows unwind code (in the Cargo registry) and its changelog
goes to a `reader` subagent, which reports how a forced unwind crosses the stack switch on x86_64
Windows and any fix in a later 0.3 release, as `file:line` and one line each.

- [x] Narrow the crash, one step at a time, each reverted after: `opt-level = 1` for the proof
  profile, `-C force-frame-pointers=yes`, `#[inline(never)]` on `nvs_runtime::run_task`
  (`crates/nvs-runtime/src/abi.rs:962`) and on the entry closure's body, and an `eprintln!` in
  each `Drop` the unwind runs. Record in the handoff which step makes the crash go away.
  `crates/nvs-host/src/scheduler.rs:931`
- [x] Name the cause in the handoff, with the evidence for it, before changing code.
- [x] Fix it as § *Standing decisions* allows, and write the cause and the fix in `Task::force_unwind`'s
  doc comment. `crates/nvs-host/src/scheduler.rs:697`
- [x] Run `cargo test --profile proof --test forced_unwind` five times: green every time. Then
  `cargo test --test forced_unwind`, and `bun nv linux-test` for the same test.
- [x] Run the stage-1 baseline's proof-profile tests again: no new red.
- [x] Write `a_server_stopped_with_a_request_still_parked_exits_cleanly` beside the server shutdown
  tests: a server with a request parked past the drain is stopped, and the process exits with its
  normal code. Run it on the proof profile. `crates/nvs-cli/tests/one_binary.rs:1`
- [ ] Add the CI step after the proofs job's build, measure what it adds on Windows, and apply the
  3-minute decision. `.github/workflows/ci.yml:583`
  - [x] The step is in the proofs job, after the build (`23ff19f1d`).
  - [ ] Its time on `proofs / windows-x86_64` is read from the next push's run, and the 3-minute
    decision applied.

### 3. nvs run's stop cancels a queue worker still connecting, and every proof program is green on the proof binary

**Does:** pins the queue worker's behaviour, then re-applies 63a6861d4 and checks it on the proof
binary and the whole proof corpus.

**Split:** an `Explore` subagent lists the tests that pin a stop with a claimed job, a drain, and
`nvs run --count`, as `file:line` and the test name, before the first item.

- [x] Each of the three behaviours in § *Standing decisions* point 5 has a test. Write any that is
  missing in `crates/nvs-cli/tests/check_command.rs`, and run it once by name.
  `crates/nvs-cli/tests/check_command.rs:377`
- [x] Re-apply 63a6861d4's change to its four files (`git show 63a6861d4` is the patch, applied with
  `bun nv splice --patch`): `Workers` keeps the tasks of workers still connecting, and `stop` cancels
  them. `crates/nvs-cli/src/worker.rs:208`
- [x] Run `run_ends_without_waiting_for_a_queue_worker_still_connecting` in debug and on the proof
  profile, and the tests of the first item again.
- [x] Time one parked example from the repository root with the proof binary, the old way and the new
  way, and record both in the handoff (the first fix measured 2050 ms and 155 ms).
- [x] Run `bun nv debt`. Every proof program is green, and the floor is no redder than the stage-1
  baseline. Met as § *Standing decisions* says: the reached groups sampled on the proof binary here,
  the rest by the goal end's `floor-debt` check.
- [x] Build the release binary (`cargo build --release --bin nvs`) and run one example from the
  repository root with it: exit 0, no crash.

### 4. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] Run `bun nv verify --doc`, `bun nv owners --closes forced-unwind` and `bun nv ci --check`, and
  fix every red.
- [ ] Push with `bun nv ci --push` and watch with `bun nv ci --watch` until every job is green,
  macOS and aarch64 included. Record `proofs / windows-x86_64`'s duration in the handoff.
  `tools/nv/cmd/ci.ts:1`

## Handoff

### State

Stages 1 and 3 are done. Stage 2's crash is named and fixed (`6b0bc591c`), its CI step is in
(`23ff19f1d`), and the server test is in (`502d3e295`). Left in stage 2: the CI step's Windows timing,
which is read at stage 4's push because nothing since `b88ffd667` is pushed yet.

**Stage 3** (`d6dd4e441`, `45e4ac4c7`): the cancel is back, and point 5's tests are
`check_command.rs:417` (new), `worker.rs:2571`/`:2598` and `check_command.rs:377`, green in debug
and proof. `run_ends_without_waiting_for_a_queue_worker_still_connecting` exits `0xC0000005` on proof
with `native_frame` inlined, so it reaches the crash. A drain never calls `Workers::stop`, so the
server test is not extended; a reload that removes workers does call it, and no test covers that.
Timing on the proof binary, root `nvs.toml`, a 300 ms sleep: 371 ms without the cancel, 375 ms
with. A refused local connect is instant here today, so the old 2 s tail does not reproduce.
`Core\Queue`, `Core\Queue\Stats`, `Core\Time` and `Core\Html` proofs are green on proof; the release
binary runs with exit 0.

**Cause and fix** are in `Task::force_unwind`'s doc (`crates/nvs-host/src/scheduler.rs:733`).
**Baseline:** on proof, `limits` is red in 7 of 16 (`limits.rs:417`, `:484`, `:524`, `:564`, `:662`,
`:819`, `:1116`), not this goal's. The sweep's reds were stale: no `env-secret-proofs` check exists,
and nightly `41428598621` was cancelled by a push.

### Next

**Stage 4, the gate, from its first item.** Stage 2's one open item is read at this stage's push.

- [ ] `tools/looper/main.ts:1` `looper verify` with `run_in_background`, every failure fixed in one
  pass, then once more.
- [ ] `tools/nv/cmd/ci.ts:1` `bun nv verify --doc`, `bun nv owners --closes forced-unwind`,
  `bun nv ci --check`.
- [ ] `tools/nv/cmd/ci.ts:1` Push with `bun nv ci --push`, watch with `bun nv ci --watch`. Read what
  the `Teardown of a cancelled task (proof profile)` step adds on `proofs / windows-x86_64`
  (`.github/workflows/ci.yml:591`); over 3 minutes, move the five cases as § *Standing decisions*
  says. Tick stage 2's last item and set stage 2 `done`.

Red now: the seven `limits` tests on the proof profile, which are the baseline and not this goal's.
Nothing else is expected red.

### Backlog

- **To file upstream; the user files it, because it publishes outside the repository.** For rustc/LLVM:
  on `x86_64-pc-windows-msvc`, a cleanup funclet of an `extern "sysv64-unwind"` function saves only
  SysV's callee-saved registers. The MSVC runtime calls it as Win64, so `rsi`, `rdi` and
  `xmm6`–`xmm15` are clobbered. For corosensei: `coroutine_func` inlines the user closure and exposes
  it, and an out-of-line call fixes it there.
