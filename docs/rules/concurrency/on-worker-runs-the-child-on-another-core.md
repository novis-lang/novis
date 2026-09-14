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

**Which entry crosses is a fact about the spawn's form.** A `Class::method(...)` entry is a label the
compiled unit's class table carries, and every core reads that unit, so the far core can prepare the
child for itself. A path entry is compiled by a resolver only the thread the process booted on holds, so
a child written as a path runs on its parent's core and the placement buys it nothing;
`crates/nvs-host/src/placed.rs`'s `# Known gaps` is the one home of what closing that takes.

Which core it is, is decided by the set that offers one, and **worker cores are started lazily and
bounded at the core count**: a core's scheduler thread starts the first time a program places a child on
one, so a program that places none has no thread, exactly as a worker with no blocking work has no pool
threads. `nvs serve` is no exception today — a serving core does not offer itself as a destination, so a
placement there starts one of these cores rather than reaching the sibling serving core ADR 0184 § 5
argues for, which is the destination set that record's *Revisiting* names. Either way the cost is one
scheduler thread per core at most: O(cores), never O(requests served), and which core ran a child is not
readable from the child, so the destination set is the only thing that changes when a serving core
registers one.
