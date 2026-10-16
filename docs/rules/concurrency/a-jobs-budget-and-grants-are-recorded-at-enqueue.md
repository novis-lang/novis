`push` takes the job's `limits` — the memory, CPU and time its isolate may spend — and its `grants`,
and both are recorded with the row. The grants are **narrowed** from those of the context that
enqueued the job and never widened: a request that could not reach a resource cannot enqueue work
that reaches it either, which is what keeps a queue from being a privilege-escalation seam.

The budget is what makes an overrun a bounded failure rather than a fatal
(`rule:concurrency/a-budget-overrun-is-a-failed-attempt`), and the grants are asked at the same
capability door every other isolate goes through
(`rule:security/capability-check-at-the-door`), with the job's own path as the scope.
