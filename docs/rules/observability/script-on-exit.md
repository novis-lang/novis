`Core\Script::onExit(callable $hook): void` registers a hook that runs as the **last user code of
the script**. Registration is request-local and runs nothing; hooks run FIFO in registration order,
and a hook that registers another hook adds it to the tail of the same drain. There is no cap beyond
the request's own memory budget: a registration is an ordinary value on the request heap, so it is
attributable and capped like any other.

Each hook receives one readonly `Script\ExitReport`, or may declare no parameter at all:

```nvs
enum Script\ExitReport::Reason { Normal, ExitCall, UncaughtThrow, Finish }

Script\ExitReport::reason(): Script\ExitReport::Reason
Script\ExitReport::status(): int
Script\ExitReport::error(): ?Throwable
Script\ExitReport::memoryPeak(): int
```

Four accessors and not four properties, because a `Core`-owned instance has no property a program
can reach (`rule:core-api/shape-rules`). Readonly is then structural: nothing writes a slot and no
program can construct one, so the only thing that builds a report is the ending itself. `status` is
the status the process will exit with; `error` is the live `Throwable` for `UncaughtThrow` — the same
object `rule:errors/on-uncaught-throw`'s handler gets — and `null` otherwise. `ExitCall` carries the
suffix so the case never shares a spelling with the `exit` keyword.

`memoryPeak` is the request's high-water mark
(`rule:observability/a-memory-peak-is-recorded-not-asked-for`), and it is here so that a one-line
hook logs the number a reading taken at this same moment would not show:
`Core\Budget::memoryHeld()` at the end of a script has already fallen back toward the baseline. It
costs a script that registers no hook nothing, and it never reaches a `FATAL` — which needs no
compensating mechanism, because a breach is the one case `rule:errors/on-limit`'s report already
names both numbers for.

Which endings drain the queue is `rule:observability/three-endings-fire-the-exit-queue`; the two that
never do are `rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`. This is the
one hook for work that must run at every ending that is not a fatal.

**Memory:** per request, O(registrations). Each hook and everything it captures stays live until the
end of the script, so a large capture registered early is held for the whole request. A script that
registers no hook pays nothing.

**Declined:**
- **Firing on a `FATAL` too, as PHP's shutdown functions do.** Arbitrary user code after a limit
  breach is how a cap stops being one
  (`rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`).
- **A bare `array<string>` report, the shape `Core\Fatal::onLimit` takes.** That shape is priced for
  a handler on a reserved slice after a memory `FATAL`, where a typed object is unaffordable. Here the
  ordinary budget applies, and an `array<string>` cannot carry the live `Throwable`.
- **A root `try/finally` and no hook at all.** It never sees `exit`
  (`rule:observability/three-endings-fire-the-exit-queue`).
- **Letting a hook set the status or swallow the error.** That rebuilds `set_exception_handler`
  (`rule:observability/a-hook-observes-and-never-steers`).
