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

The waker is the only thread-safe type the drive touches. A protocol that wants more
than one future per connection does not get an executor from this loop; it reopens the choice of HTTP
implementation instead.

- **Memory:** per connection, one shared flag and wake slot, one waker, at most one outstanding wake
  permission, and the future itself on the coroutine's own stack. That is O(in-flight), and nothing
  is allocated per poll or per park.
- **Latency:** a park costs what every other park in the host costs. The one charge over a
  hand-written state machine is the poke delivered when the last permission is given up, once per
  drive and not once per wait.

**Declined:**
- **An executor: a per-core ready queue of futures.** It is two schedulers on one core, and a
  work-stealing question that thread-per-core exists to avoid. Concurrency inside one connection is
  already a task from the scheduler.
- **A socket read that suspends the coroutine instead of answering pending.** It removes the waker
  and deadlocks: the read and write halves of one connection are one future, so a read waiting for
  the peer holds back the response that would make the peer send more. It passes only exchanges
  small enough to fit a socket buffer.
- **A third-party `block_on`.** It parks the thread, which is right off a core and wrong on one
  (`rule:concurrency/the-park-route-is-chosen-once`), and it would still need this runtime's
  cross-core wake behind its waker.
