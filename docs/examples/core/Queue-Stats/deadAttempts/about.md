Counts how many times the failed jobs of one queue were started before they failed for good.

`Core\Queue::stats` returns a `Core\Queue\Stats`, and `deadAttempts()` is one of its numbers. A job
that fails too often moves to the dead-letter table (a table of jobs that no worker runs again).
`deadAttempts()` adds up the attempts of the jobs of the queue in that table. A queue with no failed
jobs returns `0`.

Every attempt is counted in `attempts()` or in `deadAttempts()`, never in both. When a job moves to
the dead-letter table, its attempts move with it. Add the two numbers to get every attempt of the
queue.

`deadLettered()` counts the failed jobs. Divide `deadAttempts()` by it to see how many attempts a job
uses before it fails for good.

**The examples below** count the attempts of failed jobs, compute the attempts per failed job, and
print a report of failed work for several queues.
