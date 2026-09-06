A connection is one future, and the coroutine that accepted it drives that future to completion in a
loop of *clear the flag, poll, park*. The coroutine already exists — one per accepted connection —
so what this adds to the runtime is a loop and not a second runtime.

Three properties are the whole of what "not an executor" means here. There is no queue of futures and
no spawn: a future that wants concurrency asks the scheduler for a task, exactly as `Core\Task` does.
A future is polled only on the stack that owns it, so everything beneath it that is not thread-safe —
the request context, the arena, a non-atomic refcount — stays exactly as sound as it was before the
seam existed. And a pending poll suspends the **task** and not the thread: the core goes back to its
run queue and serves its other connections.

The risk this closes is not that a poll loop is hard to write. It is that a poll loop grown carelessly
becomes an executor — a ready queue, a spawn, work stealing — and then there are two schedulers on
one core, each with its own idea of which stack may touch an arena.
