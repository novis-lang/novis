A `push` issued while a transaction is open on a *different* connection than the queue's is not
transactional. The job commits on its own, the business write commits on its own, and there is a
window between them — the failure mode
`rule:concurrency/enqueue-commits-with-your-write` exists to remove.

Nothing at compile time can see this: the queue's connection name is operator-owned configuration and
the program never writes it, so the checker has no way to compare the two. The runtime therefore
**records** it, and `stats` reports non-transactional enqueues, so a deployment that has quietly lost
the property can find out by reading a counter rather than by losing a job.
