`overlap` decides what a fire does when the previous run of the **same entry** is still going, and
the count it is asked of is that entry's own running fires — never a process-wide tally, or a nightly
report could suppress an hourly one.

- **`"skip"`** (the default) drops the fire and logs it. A job still running when the next tick
  arrives is behind, and starting a second copy makes it further behind.
- **`"queue"` holds at most one pending run.** When the run finishes and a fire is pending, it starts
  immediately — the wait is on the run *ending*, not on the next minute. A second overlap while one is
  already pending is dropped and logged, not held: an unbounded pending queue in front of a
  non-durable executor is what `rule:concurrency/deferred-is-bounded-by-two-directives` refuses to
  build, and it would be no better here.
- **`"kill"`** cancels the running isolate at its next safepoint, waits for its teardown, then starts
  the new run. Cancellation runs no user code (`rule:concurrency/cancellation-runs-no-user-code`).

A word that is none of the three refuses the boot; the key has a default, and a mode the operator
asked for and will not get is refused rather than corrected. A dropped or held fire rearms like any
other, so a job that runs long falls behind by intervals rather than by copies.
