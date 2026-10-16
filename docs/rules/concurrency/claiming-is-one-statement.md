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
advice. A worker that dies inside SQLite's immediate transaction leaves it unfinished and the row
pending, so the same timeout makes the job claimable by the next worker.

- **Memory:** nothing per job on the request path, and no allocation the queue was not already making.
  The immediate transaction is the depth counter the connection already carries. On SQLite the claim
  costs one transaction's round trips where PostgreSQL costs one statement, and those round trips are
  function calls into a file rather than messages over a socket.

**Declined:**
- **A locking clause on SQLite for parity with the other dialects.** Parity of shape between dialect
  arms is not a goal; parity of guarantee is, and the single writer already gives it.
- **A deferred transaction with a retry loop around `SQLITE_BUSY`.** It keeps one `begin` and pays
  for it with a busy-error path on every claim, which the busy timeout was meant to cover already.
- **A single `update … where id = (select …) returning …` claim on SQLite.** The claim there is the
  same select-then-update pair the other two-statement dialect runs, inside the immediate transaction.
- **A build option for a wider SQLite grammar or a concurrent-write mode.** A queue statement that
  parses only under a non-default SQLite build stops parsing the day the dependency moves. A SQLite
  that admits two writers at once would also make the single-writer argument above false.
- **A lease table, an advisory lock or a coordinator process**, on any backend. Contention is the
  database's to settle, and SQLite, where the database settles it most completely, is the last place
  for an exception.
