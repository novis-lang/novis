On the throw path the order is fixed: the unwind's `finally` blocks, innermost first; then
`rule:errors/on-uncaught-throw`'s handler; then this queue; then native teardown. The failure hooks
run first so a misbehaving queue cannot starve the failure report, and a faulting handler changes
nothing — the queue runs either way. A `Core\Script::finish()` takes the throw path without being a
failure: every `finally`, innermost first, then the queue, then teardown, and no failure hook. On the
other two endings there is no ladder step: the last statement (or the `exit`), the queue, teardown.

Under every hook the heap is fully alive and output goes wherever the script's output was already
going. The temporary-directory sweep (`rule:core-classes/temporary-dir-sweep`) runs after the queue,
because a hook is user code that may still hold a path.

On the request path the queue is ordinary request code and delays the end of the response by what it
costs. Work that should not delay the response is `afterResponse`
(`rule:concurrency/after-response-outlives-the-connection`), unchanged; on a CLI run the queue runs
before the deferred work, for the same reason. The four mechanisms route by one line each: `finally`
is scoped cleanup, `afterResponse` is post-response work, `Core\Fatal` observes a failure the request
cannot survive, `onExit` is the end of the script.
