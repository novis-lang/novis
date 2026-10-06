`Core\Script::onExit(callable $hook): void` registers a hook that runs as the **last user code of
the script**. Registration is request-local and runs nothing; hooks run FIFO in registration order,
and a hook that registers another hook adds it to the tail of the same drain. There is no cap beyond
the request's own memory budget: a registration is an ordinary value on the request heap, and the
hooks and their captures stay live until the end of the script — per request, O(registrations).

Each hook receives one readonly `Script\ExitReport`, or may declare no parameter at all:

```nvs
enum Script\ExitReason { Normal, ExitCall, UncaughtThrow, Finish }

Script\ExitReport::reason(): Script\ExitReason
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
never do are `rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook`. This is the home of
PHP's `register_shutdown_function` for every ending that is not a fatal.
