Counts how many times the jobs of one queue have started.

`Core\Queue::stats` returns a `Core\Queue\Stats`, and `attempts()` is one of its numbers. Each time a
worker starts a job, the job uses one attempt. `attempts()` adds up the attempts of every job of the
queue in the jobs table. Waiting, running, finished and cancelled jobs are all counted. A new job has
not started yet, so a queue of new jobs returns `0`.

A job that fails too often moves to the dead-letter table, and its attempts move with it.
`deadAttempts()` counts them there. Add the two numbers to get every attempt of the queue.

If `attempts()` grows between two checks and `pending()` does not go down, jobs fail and run again.

**The examples below** count the attempts of a queue, add the attempts of failed jobs, and warn when
jobs fail and run again.
