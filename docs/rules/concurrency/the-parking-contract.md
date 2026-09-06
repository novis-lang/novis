Five rules govern a task that waits on I/O, and the first two are ordering rules a reader will
otherwise get backwards.

1. **Register, then suspend.** A task tells the reactor what it is waiting for, keyed by its own task
   id, and only afterwards parks. The reverse order is a lost wakeup: readiness arriving in the gap
   has nothing registered to record it against.
2. **A wake is a hint, not a promise.** A woken task re-tries its syscall and parks again on another
   `WouldBlock`. Spurious wakes are ordinary — a level-triggered poller reports a socket another read
   already drained, and a wake can race a task that finished — so waking an unknown id answers
   `false` rather than raising a fault.
3. **Deregister when the task ends.** Every registration under that id is dropped when the task is
   retired. A registration that outlives its request is the growth with total traffic that
   `rule:programs/memory-priority` calls a leak rather than a trade-off.
4. **The reactor blocks only when the run queue is empty and something is parked.** With work ready it
   is polled with a zero timeout, so readiness is collected without giving up the core. There is no
   fourth state.
5. **One reactor per worker**, not shared. A descriptor a request owns is registered with, and woken
   by, that request's own core — which is what keeps refcounts non-atomic.
