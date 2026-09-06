A worker claims a job with a single statement that selects the oldest due job in its queues and marks
it in the same moment: `for update skip locked` on PostgreSQL and MySQL, `readpast` on SQL Server, and
an immediate transaction on SQLite, whose single-writer model makes contention moot. The database
provides the mutual exclusion, so two instances of a fleet cannot claim the same job, and the runtime
writes no lease protocol, no heartbeat and no coordinator of its own.

A claimed job carries a **visibility timeout**: the claim is keyed on the instant it was taken, and a
worker that dies — or that overruns the window — matches no row when it tries to report, so the job
becomes claimable again. That is what makes delivery at-least-once
(`rule:concurrency/delivery-is-at-least-once`) and what makes bounded retries a rule rather than
advice.
