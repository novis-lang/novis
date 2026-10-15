A `push` issued while a transaction is open on a *different* connection than the queue's is not
transactional. The job commits on its own, the business write commits on its own, and there is a
window between them — the failure mode
`rule:concurrency/enqueue-commits-with-your-write` exists to remove.

Nothing at compile time can see this: the queue's connection name is operator-owned configuration and
the program never writes it, so the checker has no way to compare the two. The runtime therefore
**records** it, and `stats` reports non-transactional enqueues, so a deployment that has quietly lost
the property can find out by reading a counter rather than by losing a job.

**The case is a push with no level open on the queue's connection and one open on any other
connection the request holds**, read before the insert so a transaction `push` opens for its own
statement pair is not mistaken for the caller's. A push outside every transaction is not counted:
nothing was promised, so nothing was lost.

**The count is a column on the job row**, `non_transactional`, `1` for such a push and `null`
otherwise. It survives a restart and counts across every process writing the queue, which a
counter in process memory would not. `Core\Queue\Stats::nonTransactional` is `count(non_transactional)`
over the jobs table in the same statement as the other five counters; § 6's move to the dead-letter
table does not carry the column, so a buried job leaves the count.

**What it spends:** one nullable small integer per job row, and at each `push` a walk over the
connections the request holds. No process memory.
