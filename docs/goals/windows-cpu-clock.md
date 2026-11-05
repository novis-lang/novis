---
looper: 1
title: "a CPU limit on Windows is read from a clock as fine as Linux's"
summary: "a Windows x86-64 core's CPU time is read with QueryThreadCycleTime, finer than a millisecond and including the run in progress, at no more cost per read than GetThreadTimes, or the probe's figures show it cannot be and the rule records why"
context: [crates/nvs-host/src/cpuclock.rs, docs/rules/errors/on-limit.md]
stages:
  - id: 1
    title: "a probe on CI's Windows runner measures QueryThreadCycleTime against GetThreadTimes"
    status: open
    context: [docs/agent/playbook/running-things/a-macos-timing-red-is-measured-on-a-probe-branch.md, .github/workflows/ci.yml]
  - id: 2
    title: "a Windows x86-64 core's CPU time is its thread's cycle count, converted at a rate measured without a wait"
    status: open
    context: [crates/nvs-runtime/src/ctx/safepoint.rs, tools/nv/proofs/run.ts, tools/nv/test/proofs-time.test.ts, Cargo.toml]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: fine-cpu-clock
    stage: 2
    name: a thread's CPU clock moves in sub-millisecond steps and a read from another thread counts the run in progress
    run: [cargo, test, --lib, "cpuclock::"]
    timeout: 1800
    tests:
      - a_cpu_clock_moves_in_steps_finer_than_a_millisecond
      - a_read_from_another_thread_counts_the_run_in_progress
      - the_cycle_rate_agrees_with_the_thread_times_over_a_long_run
  - id: sweep-bound
    stage: 2
    name: the proofs runner's CPU stop bound counts a clock tick only where the clock has one
    run: [bun, test, tools/nv/test/proofs-time.test.ts]
    timeout: 300
---

# a CPU limit on Windows is read from a clock as fine as Linux's

## Why

`[limits] cpu_time` is a security bound: it stops a runaway request on its core. On Windows a core's
CPU time is read with `GetThreadTimes` (`crates/nvs-host/src/cpuclock.rs:233`). Windows does not
measure that time. At each clock interrupt, every 15.625 ms by default, the kernel charges the whole
interval to the thread that is running at that moment. So the value moves in 15.625 ms steps, and over
a short span it is a sample. A request that ran 10 ms can be charged nothing, and one that ran 1 ms
across an interrupt is charged 15.6 ms.

Two things follow today. A CPU stop on Windows comes up to one step per thread later than on Linux.
The proofs runner has to allow for that, which is the `tick_us` term `cpu_clock_tick` writes
(`crates/nvs-runtime/src/ctx/safepoint.rs:1486`). And the switch charging that keeps one request from
paying for another's loop (`rule:errors/on-limit`) is coarse by the same step on Windows.

`QueryThreadCycleTime` returns the cycles the kernel charged the thread, counted with the CPU's
timestamp counter at each context switch. It is not sampled at the interrupt, and it takes the same
handle and access right the code already opens. The user's call: take it if it makes the bound more
robust and more secure with no real performance cost. Stage 1 measures whether it does, and stage 2
builds it only if so.

## Standing decisions

1. **Measure on the real runner before building.** CI's Windows runner is a Hyper-V virtual machine,
   and the dev machine is not. The facts stage 2 rests on are measured there, on a `probe/` branch
   (the playbook entry in stage 1's `context`). The figures go into `cpuclock.rs`'s module doc, where
   the reasons for each platform's clock already live.
2. **The adoption bar.** Stage 2 builds the cycle clock only when the probe shows all four:
   - **Steps:** a thread reading its own cycle-derived time sees steps under 0.1 ms while it spins.
   - **Run in progress:** a read from another thread of a spinning thread trails that thread's own
     read by under 1 ms, at the 99th percentile of 1000 reads.
   - **Rate:** over a spin of at least 2 s, cycle-derived time agrees with `GetThreadTimes` within 1%
     plus one 15.625 ms step.
   - **Cost:** one read from another thread (`OpenThread`, the query, `CloseHandle`) costs at most
     1.2 times what the same read with `GetThreadTimes` costs, as the median of 100,000 reads.
     A thread's read of its own clock meets the same bound.
3. **If "run in progress" fails and the other three pass:** the watchdog's read of another thread
   suspends it first, as macOS does (`cpuclock.rs:46`). That is `SuspendThread`, then
   `GetThreadContext` to wait until it is off its processor, then the read, then `ResumeThread`. The
   probe measures that path's cost too, and the cost bar applies to the thread's own read only. The
   suspend's cost is stated in the module doc, as macOS's is: two context switches per core under a
   CPU ceiling, once per watchdog sweep.
4. **If "steps", "rate" or "cost" fails:** nothing is built. Stage 2's items become one: a `Declined`
   bullet in `docs/rules/errors/on-limit.md` with the figures, then the goal ends. The `fine-cpu-clock`
   check then names tests that hold for the clock as it is, and the session rewrites the check's
   `tests` list to the ones it wrote, saying why in the commit.
5. **x86-64 Windows only.** The conversion uses the timestamp counter's rate, which is a fact about
   x86-64. Any other Windows target keeps `GetThreadTimes`, and `cpu_clock_tick` returns 15.625 ms
   there and zero on x86-64 Windows. That is a `cfg`, not a run-time choice.
6. **The rate is measured without a wait.** The cycles-to-time rate comes from two pairs of
   (`_rdtsc`, `QueryPerformanceCounter`) readings at least 1 ms of wall time apart, which gives an
   error under 0.1%. Startup gets no `sleep` and no spin. The first pair is taken when the first
   `ThreadClock` is made. The rate is fixed at the first read at least 1 ms later, in a `OnceLock`.
   A read before that converts with the rate from the pairs it has so far. Safe default, if that
   cannot be made sound: a 200 µs spin when the first `ThreadClock` is made, with its cost stated in
   the module doc.
7. **One clock per platform for both readers.** A core reads its own clock at each task switch, and the
   watchdog reads it from its thread. Both read the cycle clock, so the two still share one measure
   (`cpuclock.rs:8`).
8. **The `windows-sys` feature.** `QueryThreadCycleTime` is in `Win32_System_WindowsProgramming`, which
   the workspace does not enable yet (`Cargo.toml:245`). Enabling it is this goal's to do. It adds no
   dependency.
9. **Rejected, and recorded as `Declined` in `rule:errors/on-limit`:** `timeBeginPeriod(1)` or
   `NtSetTimerResolution`. They make the clock interrupt fire every millisecond for as long as the
   server runs, which costs power on the whole machine. Windows 10 2004 and later also limit what one
   process's request changes, and nothing shows that `GetThreadTimes` gets finer with them.
10. **The probe branch is pushed and deleted by the session.** It is this repository's own remote, so
    it is not a publish. Push `probe/windows-cpu-clock` from a worktree under `.agent-tmp/`. Its own
    workflow file exists only on that branch and runs only the probe, on `windows-latest`. Delete the
    branch with `git push origin --delete probe/windows-cpu-clock` once the figures are read, and
    remove the worktree. Never a force push.
11. **Lanes:** one. Stage 2 builds on stage 1's figures, so the stages run in order.

## Stages

### 1. a probe on CI's Windows runner measures QueryThreadCycleTime against GetThreadTimes

**Does:** answers decision 2's four questions, plus decision 3's suspend cost, with figures from CI's
Windows runner. Nothing lands on `main` but the figures, in `cpuclock.rs`'s module doc.

**Split:** a `reader` subagent reads the probe run's log and returns the figures as a table.

- [ ] Write the probe as `#[ignore]` tests in `cpuclock.rs`'s test module, on the probe branch only.
  They print figures with `--nocapture` and assert nothing. `crates/nvs-host/src/cpuclock.rs:428`
  - steps: spin and sample the thread's own clock for 200 ms, with both calls; print the distinct
    step sizes.
  - run in progress: thread B spins and publishes its own read, and thread A reads B through
    `OpenThread`. Print the 50th, 99th and maximum lag over 1000 reads, for both calls.
  - rate: two (`_rdtsc`, `QueryPerformanceCounter`) pairs 1 ms apart and 20 ms apart, and a 2 s spin
    compared with `GetThreadTimes`.
  - cost: the median nanoseconds of 100,000 reads of each kind, own thread and another thread. Also
    the suspend path of decision 3.
- [ ] Run the probe once on this machine with
  `cargo test --lib cpuclock::tests::probe_ -- --ignored --nocapture`, so the branch is not pushed to
  find a compile error.
- [ ] Push `probe/windows-cpu-clock` with its own workflow running that one command on
  `windows-latest`, and run it twice. Read the figures, not pass or fail. `.github/workflows/ci.yml:1`
- [ ] Write the figures and the decision 2 verdict into `cpuclock.rs`'s module doc, in the paragraph
  on Windows. `crates/nvs-host/src/cpuclock.rs:27`
- [ ] Delete the probe branch and its worktree (decision 10).
- [ ] If decision 4 applies: write the `Declined` bullet into `docs/rules/errors/on-limit.md:57`,
  rewrite stage 2 as decision 4 says, and go on to the gate stage.

### 2. a Windows x86-64 core's CPU time is its thread's cycle count, converted at a rate measured without a wait

**Does:** replaces `GetThreadTimes` with `QueryThreadCycleTime` on x86-64 Windows, removes the tick
that clock no longer has, and pins both with tests that hold on every platform.

- [ ] Enable `Win32_System_WindowsProgramming` in the workspace's `windows-sys` features
  (decision 8). `Cargo.toml:245`
- [ ] The rate of decision 6: the first pair when the first `ThreadClock` is made, the rate fixed in a
  `OnceLock` at the first read 1 ms or more later. `crates/nvs-host/src/cpuclock.rs:104`
- [ ] Windows x86-64 `burned` reads `QueryThreadCycleTime` and converts it with the rate. If decision 3
  applies, a read of another thread suspends it first. Other Windows targets keep `GetThreadTimes`
  (decision 5). `crates/nvs-host/src/cpuclock.rs:233`
- [ ] `cpu_clock_tick` returns zero on x86-64 Windows and 15.625 ms on other Windows targets, and its
  doc says why. `crates/nvs-runtime/src/ctx/safepoint.rs:1486`
- [ ] The proofs runner keeps its `tick_us` term, and `proofs-time.test.ts` gains a case where a
  zero tick adds nothing. `tools/nv/proofs/run.ts:404`, `tools/nv/test/proofs-time.test.ts:24`
- [ ] The three tests the `fine-cpu-clock` check names, in `cpuclock.rs`'s test module. Each waits
  on the clock it tests, never on wall time (`docs/agent/testing.md` § *A test holds on every CI
  runner*). `crates/nvs-host/src/cpuclock.rs:428`
  - `a_cpu_clock_moves_in_steps_finer_than_a_millisecond`: spin and read the thread's own clock until
    it has moved ten times. Every step is under 1 ms. Every platform.
  - `a_read_from_another_thread_counts_the_run_in_progress`: thread B spins until its own clock shows
    20 ms, publishes that reading and keeps spinning. A read of B from another thread is at least that
    reading. Every platform.
  - `the_cycle_rate_agrees_with_the_thread_times_over_a_long_run`: spin until `GetThreadTimes` shows
    500 ms. The cycle-derived time agrees within 1% plus one 15.625 ms step. x86-64 Windows only.
- [ ] Run the `fine-cpu-clock` check's command once: green.
- [ ] The Windows paragraph of `cpuclock.rs`'s module doc says which call each target uses, what a read
  costs, and what the rate costs at startup. `crates/nvs-host/src/cpuclock.rs:27`
- [ ] `rule:errors/on-limit` says a core's CPU clock reads finer than a millisecond on every platform
  that enforces the ceiling, and gains the `Declined` bullet of decision 9.
  `docs/rules/errors/on-limit.md:38`, `docs/rules/errors/on-limit.md:57`

### 3. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built. The proofs runner already counts one 15.625 ms tick per thread on Windows
(`1ce15b6b3`), which is the stopgap this goal replaces.

### Next

Stage 1, `crates/nvs-host/src/cpuclock.rs` and a probe branch. No gate.

- [ ] The probe tests in `crates/nvs-host/src/cpuclock.rs:428`, run once here, then twice on
  `probe/windows-cpu-clock`.
- [ ] The figures and the verdict in the module doc. `crates/nvs-host/src/cpuclock.rs:27`
- [ ] The probe workflow beside `.github/workflows/ci.yml:1` on the branch only, then the branch and
  its worktree deleted.

### Backlog
