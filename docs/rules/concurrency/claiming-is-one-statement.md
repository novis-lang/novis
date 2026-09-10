A worker claims a job with a single statement that selects the oldest due job in its queues and marks
it in the same moment: `for update skip locked` on PostgreSQL and MySQL, `readpast` on SQL Server, and
an immediate transaction on SQLite, whose single-writer model makes contention moot. The database
provides the mutual exclusion, so two instances of a fleet cannot claim the same job, and the runtime
writes no lease protocol, no heartbeat and no coordinator of its own.

**On SQLite the exclusion is that transaction itself, so the claim carries no locking clause and
cannot be given one.** That backend has a single writer: inside an immediate transaction no second
connection is writing at all, so two workers cannot come back with one row — which is what `for
update skip locked` buys on a backend that has row locks and concurrent writers to need them from.
The worker that arrives second waits for the write lock, and by the time it proceeds the row the
first one took is no longer due. `skip locked` is a syntax error there, and the reason not to reach
for one is that there is nothing left for it to do. The transaction has to be an *immediate* one
rather than the deferred transaction a bare `BEGIN` opens: a transaction that reads a row and then
writes it back asks to upgrade a shared lock, and SQLite refuses an upgrade without honouring the
busy timeout, so a deferred claim fails under exactly the concurrency it exists to survive.

A claimed job carries a **visibility timeout**: the claim is keyed on the instant it was taken, and a
worker that dies — or that overruns the window — matches no row when it tries to report, so the job
becomes claimable again. That is what makes delivery at-least-once
(`rule:concurrency/delivery-is-at-least-once`) and what makes bounded retries a rule rather than
advice.
