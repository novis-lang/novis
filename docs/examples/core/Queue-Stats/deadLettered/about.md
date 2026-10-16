Counts the jobs of one queue that failed for good.

`Core\Queue::stats` returns a `Core\Queue\Stats`, and `deadLettered()` is one of its numbers. A job
that fails more times than its `maxAttempts` allows moves to the dead-letter table (a table of jobs
that no worker runs again). `deadLettered()` counts the jobs of the queue in that table. A queue with
no failed jobs returns `0`.

A failed job stays in the dead-letter table until you delete it. `Core\Queue::purge` with
`State::Dead` deletes these jobs, and `Core\Queue::delete` deletes one job. Only then does the number
go down.

These jobs did not finish their work, so `deadLettered()` is a good number to alert on. The number of
attempts they used is `deadAttempts()`.

**The examples below** count the failed jobs, show that they stay until you delete them, and send an
alert when more jobs failed since the last check.
