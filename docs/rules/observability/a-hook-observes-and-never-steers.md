The report — reason and status — is fixed before the first hook runs, and nothing a hook does changes
it. Anything else rebuilds `set_exception_handler`, which the escalation ladder replaced.

A hook that throws is abandoned where it stands, its `Throwable` is written to `Core\Log` with the
request's trace id rather than swallowed (`rule:errors/log-write`), and the queue continues with the
next hook. `exit` inside a hook throws `RuntimeError` at the call site: the alternative hands any hook
a way to suppress every hook behind it and to name a status the report never carried. A hook may
register another hook; it joins the tail of the same drain.

Only one thing stops the drain early: a limit breach inside a hook is a `FATAL`, and
`rule:observability/a-fatal-and-a-cancellation-run-no-exit-hook` takes over from there.
