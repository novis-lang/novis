A queue worker that finds no due job waits one second before it asks the database again, and a
`Core\Queue::push` in the same process wakes it as soon as the job is committed. A worker asks the
moment it starts and again the moment a job is written back, so the wait is only ever taken by a
worker with nothing to do. It is one fixed interval with no backoff and no `[queue]` key, and it is
the whole of what a `[queue]` with nothing in it costs: one statement per worker per second.

**The wake follows the commit, never the statement.** A worker woken before the job is committed asks
for a row no other connection can see, finds nothing and goes back to waiting, so:

- a push outside a transaction is its own committed statement, and the wake follows it
  (`rule:concurrency/enqueue-commits-with-your-write`);
- a push inside a `transaction` on the queue's own connection wakes when that transaction commits, at
  the outermost commit when transactions nest, and a rollback wakes nobody;
- a push while a transaction is open on a different connection commits on its own, so the wake follows
  the statement (`rule:concurrency/foreign-connection-enqueue-is-counted`).

**A stop wakes a waiting worker too**, under `nvs run` and under `nvs serve`. The script's exit, a
reload that takes a worker away and the drain each end the wait at once, so neither a run's exit nor a
shutdown waits out the second (`rule:concurrency/one-process-serves-requests-schedules-and-jobs`).

**Only work this process announced is woken for.** A job pushed by another process, a delayed job
coming due, a retry after its backoff and a job reclaimed after its worker died are found by the next
ask, so each may start up to one second late. A transaction a program opens in its own statement text
is one the runtime does not see end, so a push inside one is found the same way.

**The mechanism is the process's own, and it is the same on every driver.** The wake is a call this
process makes after its own statement or its own commit, carried to the core the workers run on by the
runtime's cross-thread wake (`rule:concurrency/a-handle-given-up-by-its-task-wakes-nothing` is what
lets a worker take one per wait). It is never a message a database sends: there is no `LISTEN` and
`NOTIFY` and no other backend's notification channel, so a backend that has none behaves exactly as
one that has, and there is one path to test (`rule:concurrency/no-broker-and-no-driver-interface`).
