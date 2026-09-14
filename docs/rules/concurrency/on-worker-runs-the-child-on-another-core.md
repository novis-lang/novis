A child spawned `on: "worker"` is **started** on a core other than its parent's, and *started* is the
whole of the placement: a task never migrates (`rule:concurrency/a-wake-never-moves-a-task`), so which
core runs it is decided once, where it is spawned, and is never revisited while it runs. Everything else
about it is unchanged — it is an isolate with the boundary
`rule:security/isolate-shares-nothing` describes, and `on: "here"` is the same child on the parent's own
core.

What crosses is what `rule:security/isolate-values-cross-by-copy` allows, with one thing subtracted: the
**move is not available**. A refcount is non-atomic because a value is reachable from one core only, so
the argument is copied at every node on the way in and the answer at every node on the way out, and a
large result is an argument for leaving the child on the parent's core rather than for a cheaper
crossing.

The mechanism is the blocking pool's, turned around (`crates/nvs-host/src/blocking.rs`). The parent takes
a `RemoteWake` for itself, puts the compiled unit, the copied argument and that handle into the
destination core's **inbox**, pokes that core's poller, and parks. The other core's reactor drains the
inbox and starts the child as a root task on its own scheduler; when the child finishes, its answer is
copied into the handle's slot and the handle is dropped, which ends the parent's park. What crosses the
thread boundary is therefore still an id and a poke — the slot is the record, and the wake only ends the
wait.

A worker-placed child is its parent's child in every other respect. It is cancelled when the parent is
cancelled, it is charged to the tree's budget rather than to a per-call limit
(`rule:security/isolate-budget-is-the-trees`), and
`rule:concurrency/nothing-is-still-running-when-a-call-returns` holds across the thread: the parent's
call does not return until the cancellation it sent has been acknowledged from the other core and the
child's own teardown has run there.

Which core it is depends on what the process is. Under `nvs serve` it is a sibling serving core, chosen
round-robin, because those cores and their schedulers already exist and a second thread per core would
oversubscribe the machine. Everywhere else — a `nvs run`, a job, a test, where one scheduler is all there
is — **worker cores are started lazily and bounded at the core count**, so a program that places no child
on one has none, exactly as a worker with no blocking work has no pool threads. Either way the cost is
one scheduler thread per core at most: O(cores), never O(requests served).
