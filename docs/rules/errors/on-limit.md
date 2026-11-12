`Core\Fatal::onLimit(callable(LimitReport): void $handler): void` fires only for a resource-limit
`FATAL` — memory, CPU time, `max_output`, wall time, `max_script_depth` and call-stack depth. An
internal panic never reaches it (`rule:errors/panics-bypass-user-code`).

Registration is request-local, living beside the pending-error slot in `Ctx`, and dies with the
request like every other per-request slot. The handler runs on a **reserve carved out of the
request's own budget at request start** and unavailable to ordinary execution — otherwise a request
that exhausted its memory would have nothing left to report with.

There are **two reserves, memory and time, not one per limit**, because those are the only two
resources a handler cannot run without spending; a `max_output` breach refuses the handler nothing.
Sizing them is a `System`-class decision, not the script's: a program choosing the size of its own
safety net is exactly the case where the choice should belong to someone else.

`LimitReport` is an array rather than a class. The report is built where the breach happens, in
`nvs-runtime`, which holds no `Core` class descriptor to instantiate one from — and a keyed array
takes a later field without changing the signature of a handler already written.

**Memory:** the memory reserve is a ceiling, not an allocation. Ordinary execution runs under
`[limits] memory` less the reserve, which is 1 MiB by default and never more than a quarter of the
ceiling. Every configured run has a memory ceiling, 128 MiB where none is written
(`rule:config/no-configuration-file-is-a-complete-configuration`), so the reserve is carved by
default; a request whose `memory` is `false` reserves nothing.

**Time:** the CPU reserve is carved out of `[limits] cpu_time` the same way: `fatal_reserve_time`,
50 ms by default and never more than a quarter of the ceiling. A handler entered for a wall-time stop
also gets a wall-time slice of that size, sized against `[limits] wall_time`. Both are counted from
the handler's entry, never from the moment of the breach. The CPU slice is the tree's spend at entry
plus the reserve, published to the sampler. The wall slice is an instant the tree carries. The poll
lowers a wall-time raise that lands inside it, and the first raise past its end stops the handler.
The poll lowers a CPU raise inside the handler while the sampler's reading of the tree is under the
widened ceiling: a sweep that read the ceiling from before the handler's entry raises after the
handler lowered the flag. So a request thread that reaches its poll late, or is preempted, and a
sampler preempted between its read and its raise, do not take the slice: a handler's time does not
depend on the host's speed. The cost is one word and one byte per tree, a store at each end of a
handler, and a clock read for each raise inside the slice. A wall-stopped request may run past its
`wall_time` by its slice plus one sweep interval.

A ceiling is enforced where the resource is spent, not where a program happens to look, so a loop
that allocates nothing and calls nothing is still stopped. The watchdog thread, which already wakes on
an interval and holds an entry per core, reads each core's own CPU clock and raises the CPU bit in the
word compiled code polls at every back edge; the allocator raises the memory bit in the same word
(`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`). That word is reached by
address rather than living inside `Ctx`, because neither writer holds a context and a write through one
would alias the `&mut Ctx` a helper body holds. A request tree shares one word, as it shares one budget
(`rule:security/isolate-budget-is-the-trees`), so a child made after a flag was raised is not born
clean. A core charges each request only for the time that request's own tasks run, taking a reading
of its clock at each task switch between requests, so a request that waits and then loops is
stopped like one that never waited, and the requests waiting beside it are charged nothing for the
loop. A runaway request therefore stops itself at its next poll, and the core it ran on is never
wedged.

**A parked request is ended at its wall deadline by the park funnel**, because a parked task reaches
no poll and the watchdog's raise reaches only a task that runs. Every park goes through `yield_on`
(`crates/nvs-host/src/scheduler.rs`), and for a `Waiting::Parked` it reads the bound of the tree whose
turn the stack is: the open handler slice's end, else the wall deadline, else none
(`ArmedStack::bound`). It files the bound as the task's one timer entry when it is earlier than the
site's own, and puts the site's entry back on resume. A wake past the bound, or a park whose bound has
passed already, claims the wall stop as the watchdog does, raises `WALL_TIME` and answers
`Resumed::Cancelled`. That has one ending: the member's `Ctx::cancel` runs the poll, which runs the
handler in its slice and ends the request with the wall-time `FATAL`, and records no cancellation. A
wait with no `Ctx`, such as a socket read inside a driver, returns its I/O error, and `run_helper`
reports a member that failed past the wall deadline as the wall-time `FATAL`, after the memory
question. No member's own catchable timeout comes first. A wait may outlive `wall_time` only by naming
its reason, `Waiting::ParkedPast(PastWallTime)`: a server task, the connection's own drive, a join of
the task's own children (each bounded itself), a blocking-pool job, a debugger stop and the request's
end. `crates/nvs-host/tests/park_sites.rs` holds each reason to its files. The cost: one load per park
for a tree with no `wall_time`, and one more load and a clock read under one. A park whose bound is
the earlier instant adds one timer insert and one removal, and a member call that fails adds one load
and, under a `wall_time`, a clock read. No memory.

A limit handler that cannot run without weakening the bound does not run: stopping the request is
security and its report is not, so the bound stays as it is (`Ctx::run_limit_handler` in
`crates/nvs-runtime/src/ctx/hooks.rs`).

**Declined:**
- **A watchdog sweep over parked tasks to end them at the wall deadline.** It costs every sweep a walk
  of every parked task, and it ends a wait one interval late. The funnel ends it on the core, at the
  instant, for the park's own timer entry.
- **A new `Resumed` answer for the deadline.** Every park site would have to handle it, and
  `Cancelled` already ends every one of them.
- **A wall clock in place of CPU time.** A wall clock cannot tell a runaway loop from a request waiting
  on a slow database, and `[limits] cpu_time` exists to stop the first and leave the second alone. A
  platform with no per-thread clock another thread may read enforces no CPU ceiling and says so at
  boot.
- **Killing the core a runaway request is wedging.** No point exists at which another thread may
  unwind a core running compiled code (`rule:http-server/a-wedged-core-is-shed-never-killed`).
- **`QueryThreadCycleTime` for a Windows core's CPU clock.** It is fine and counts the run in
  progress, but the watchdog's read of a core that is running costs 7.5 to 10 times a
  `GetThreadTimes` read, about 10 µs, measured on CI's Windows runners. The likely cause is that
  Windows interrupts the running core to read its count, so the request would pay too. A Windows CPU stop stays up to one
  15.625 ms clock step late, and the proofs runner allows that step. The figures are in
  `crates/nvs-host/src/cpuclock.rs`'s module doc.
- **`timeBeginPeriod(1)` or `NtSetTimerResolution` to make `GetThreadTimes` finer.** They make the
  clock interrupt fire every millisecond for as long as the server runs, which costs power on the
  whole machine. Windows 10 2004 and later limit what one process's request changes, and nothing
  shows that `GetThreadTimes` gets finer with them.
