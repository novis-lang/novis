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
ceiling. A request with no memory ceiling reserves nothing.

A ceiling is enforced where the resource is spent, not where a program happens to look, so a loop
that allocates nothing and calls nothing is still stopped. The watchdog thread, which already wakes on
an interval and holds an entry per core, reads each core's own CPU clock and raises the CPU bit in the
word compiled code polls at every back edge; the allocator raises the memory bit in the same word
(`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`). That word is reached by
address rather than living inside `Ctx`, because neither writer holds a context and a write through one
would alias the `&mut Ctx` a helper body holds. A request tree shares one word, as it shares one budget
(`rule:security/isolate-budget-is-the-trees`), so a child made after a flag was raised is not born
clean. A runaway request therefore stops itself at its next poll, and the core it ran on is never
wedged.

**Declined:**
- **A wall clock in place of CPU time.** A wall clock cannot tell a runaway loop from a request waiting
  on a slow database, and `[limits] cpu_time` exists to stop the first and leave the second alone. A
  platform with no per-thread clock another thread may read enforces no CPU ceiling and says so at
  boot.
- **Killing the core a runaway request is wedging.** No point exists at which another thread may
  unwind a core running compiled code (`rule:http-server/a-wedged-core-is-shed-never-killed`).
