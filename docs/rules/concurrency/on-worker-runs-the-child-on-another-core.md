A child spawned `on: "worker"` is **started** on a core other than its parent's whenever one is idle,
and on its parent's own core only when every other core is busy. *Started* is the whole of the
placement: a task never migrates (`rule:concurrency/a-wake-never-moves-a-task`), so which core runs it
is decided once, where it is spawned, and is never revisited while it runs. Everything else about it is
unchanged — it is an isolate with the boundary `rule:security/isolate-shares-nothing` describes, and
`on: "here"` is the same child on the parent's own core without the crossing.

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

**Both entry forms cross, and each is prepared by the core that runs the child.** A
`Class::method` entry is a label the compiled unit's class table carries, and every core reads that
unit. A path entry is compiled through a resolver, which is per thread — so the process publishes one
handle to the compiler it already built, and a core installs it on its own thread as it starts. What is
shared either way is compiled code and nothing else, which is the sharing
`rule:security/isolate-shares-nothing` permits. A process that publishes no resolver at all — a `nvs
check`, an embedder that installed one only on its own thread — runs a path-entry child on the parent's
core rather than placing it where nothing could compile it;
`crates/nvs-host/src/placed.rs`'s *Both entry forms cross* is the one home of what each form needs.

Which core it is, is decided by the set that offers one, and **worker cores are started lazily and
bounded at the core count**: a core's scheduler thread starts the first time a program places a child on
one, so a program that places none has no thread, exactly as a worker with no blocking work has no pool
threads. **Under `nvs serve` the set is the serving cores themselves**: each one registers its own inbox
and drains it from a receptionist beside its accept loops, so a placement reaches a sibling serving core
and no thread is started beside threads that are already pinned: a second thread per core for placement
work would oversubscribe exactly the cores a served request's latency depends on. A core that
offered itself that way stops receiving when the process begins draining and goes once the placements it
is holding have answered, because those belong to the requests the drain is there to finish. Either way
the cost is one
scheduler thread per core at most: O(cores), never O(requests served), and which core ran a child is not
readable from the child, so the destination set is the only thing that changes when a serving core
registers one.

**The order a core is picked in is: an idle core other than the parent's, a new core if the bound
allows one, the parent's own core if no placement is running on it, and then the other cores in turn.**
The parent's core is a destination because a parent that awaits its children is parked and leaves its
core free: under `nvs serve` on four cores, a request that places four children runs all four at once,
where queueing the fourth behind a busy sibling took two rounds. It comes last because a child there
shares the core with its parent. When the child the parent awaits finishes first, the parent runs
again when the child on its own core next gives the core back, since neither can move. A child that
waits gives it back at its wait, and a child that computes gives it back when its 10 ms time slice is
used up, so the parent waits one slice at most. A parent that awaits all its children loses nothing
to that, and a parent that does work between its awaits pays at most one slice per await. One placement at a time is the cap on the parent's core, so a fifth
child queues behind a sibling rather than doubling up beside its parent. A process whose one core is
the one asking runs the child where it stands, as `on: "here"` would, because the trip through its own
inbox would buy two copies and no parallelism.

A worker placement's latency floor is two copies and two cross-thread wakes, which is why the same-core
placement is the default and `on: "worker"` is for a child that is CPU-bound and whose values are small.
The inbox adds no lock to the request path: it has one consumer, the core that owns it, and the core being
asked is already asleep in a poll that the poke ends, so a start costs the same wake a blocking-pool
answer costs.

- **Memory:** per worker-placed child in flight, charged to its tree: one task context and one pooled
  stack on the destination core, plus the argument and the answer each copied at every node, twice what
  a same-core child pays. Per process: at most one scheduler thread per core, only for cores a child was
  placed on, and none under `nvs serve`. That is O(cores) and O(in-flight), and nothing grows with
  requests served.

**Declined:**
- **Never placing a child on its parent's core.** With every sibling busy the child queues behind
  another request's child while the core its own parent parked on sits idle, so a request that places
  one child per core takes two rounds instead of one.
- **Migrating the child to an idle core.** Refcounts are non-atomic and an arena belongs to one core, so
  this is unsound rather than expensive (`rule:concurrency/a-wake-never-moves-a-task`).
- **A shared global run queue every core steals from.** It is one piece of contended state on the request
  path, and it makes which core runs a child unpredictable, which makes the budget, the grants and the
  cancellation path harder to reason about for no gain. The same objection keeps the blocking pool per
  worker.
- **Work stealing between serving cores.** It makes placement implicit, and an implicit placement cannot be
  reasoned about from the spawn site, which is where a program states what it wants.
- **A thread per worker-placed child.** It is O(in-flight children) in threads rather than O(cores), and
  it defeats the pinning the thread-per-core design rests on.
- **A subprocess for a worker placement.** An OS process costs orders of magnitude more than a task
  (`rule:security/isolate-shares-nothing`).
