`push` takes the job's `limits` — the memory, CPU and time its isolate may spend — and its `grants`,
and both are recorded with the row. The grants are **narrowed** from those of the context that
enqueued the job and never widened: a request that could not reach a resource cannot enqueue work
that reaches it either, which is what keeps a queue from being a privilege-escalation seam.

The budget is what makes an overrun a bounded failure rather than a fatal
(`rule:concurrency/a-budget-overrun-is-a-failed-attempt`), and the grants are asked at the same
capability door every other isolate goes through
(`rule:security/capability-check-at-the-door`), with the job's own path as the scope.

Enqueuing is starting a script, so `push` needs `script.spawn` on the calling namespace's
`[grants]` line (`rule:security/script-spawn-capability`). The application enqueues with no line
written, and a package enqueues only where a human wrote `script.spawn` on its line. The grants
recorded with a job narrow what its isolate holds. They are not the permission to enqueue.
