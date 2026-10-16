Counts the jobs of one queue.

`Core\Queue::stats` returns a `Core\Queue\Stats` with six numbers. `pending()` is the number of jobs that wait for a worker. `claimed()` is the number of jobs that a worker is running now. `attempts()` is how many times the jobs in the jobs table have started. `deadLettered()` is the number of jobs in the dead-letter table (the table of jobs that failed too often). `deadAttempts()` is how many times those jobs started. `nonTransactional()` is the number of jobs in the jobs table that were pushed inside a transaction of a different database. Such a job is saved at once, so a rollback of that transaction does not delete it.

All six numbers are read at the same moment, so they agree with each other. A queue with no jobs returns six zeros. Finished and cancelled jobs are not waiting and not running, so `pending()` and `claimed()` do not count them.

**The examples below** count the waiting jobs, count two queues one at a time, and check the health of a queue.
