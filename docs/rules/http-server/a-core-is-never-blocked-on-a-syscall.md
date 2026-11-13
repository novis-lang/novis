The runtime is one single-threaded scheduler pinned per core, and a request never migrates. A blocking syscall on that thread stalls every coroutine pinned to the same core — the requests that lose are the neighbours, and they did nothing. A socket read is *not* one of those syscalls: the stream parks the coroutine and hands the core back (`rule:concurrency/the-reactor-reports-readiness`, `rule:concurrency/the-parking-contract`), which is the cheaper mechanism and the one to reach for whenever the kernel can report the wait.

**Filesystem calls, name resolution, and waiting on a child process go to the blocking pool**; the reactor thread issues no call that can block on external state. The pool is **bounded at twice the core count**, per worker, and never grows with requests served. The handoff costs microseconds against operations that cost milliseconds, and what it avoids is an entire core idle for the same interval.

**Static serving is one of them.** The server's file lookup and open (`OnDisk`) each hand their whole body to the pool as one job: the `stat`, the canonical path and a link's walk for a path a mount selects, and the `stat`, the open and the `fstat` for the file it sends. A file body reads each 64 KiB chunk as one job, with the file moved in and back, and its first job seeks to the range's start. That covers a static mount, the default document, a miss that reaches the entry and the file a `Core\Response::sendFile` names. What it spends: one hand-off per path asked and one per chunk sent. A request under `static = true` pays one to three hand-offs before its program runs.

**A job's CPU time is the request's.** The pool thread reads its own CPU clock before and after each job, and each answer a pinned thread gives, and the task adds the difference to its request's charge when the answer lands. `[limits] cpu_time` then counts a slow statement or a large file hashed off the core exactly as it counts the same work on it (`rule:security/isolate-budget-is-the-trees`). That is two clock reads per job, and one more word per published request tree.

**A job its task gave up on in the queue is skipped.** A task that waits with a deadline stops
waiting when the deadline passes, and marks the job abandoned. The pool thread reads that mark before
it runs the job: an abandoned job frees what it holds, releases its thread and sends its wake, and its
body never runs. So a queue of abandoned work does not hold the pool's threads past the requests that
queued it. A job that has started still runs to its end, which the bound covers. What it spends: one
uncontended lock per job on the pool thread.

The bound is stated because the default it replaces is not: an unbounded pool of threads that each reserve a stack is a footprint that grows with a workload's blocking fan-out rather than with its concurrency. What the pool spends, per `rule:programs/memory-priority`: at most that many OS thread stacks per worker, resident only in what a job touches, plus one boxed Rust `FnOnce` per job in flight.

**Declined:**
- **`io_uring` for the blocking paths.** It is faster and would remove the handoff rather than bound it,
  but it is a second I/O model on one platform only, in a workspace that keeps its runtime dependency
  surface narrow. Simplicity outweighs it until a measurement says otherwise.
