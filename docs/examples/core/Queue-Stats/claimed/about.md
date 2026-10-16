Counts the jobs of one queue that a worker is running now.

`Core\Queue::stats` returns a `Core\Queue\Stats`, and `claimed()` is one of its numbers. When a worker
starts a job, the job is claimed. It stays claimed until the job finishes or fails. Jobs that wait for
a worker are counted by `pending()`, not by `claimed()`.

A worker can stop while it runs a job, for example when its server restarts. The job is then still
counted in `claimed()`. After the `[queue] visibility` time, another worker can start the job again.

If `claimed()` equals the number of jobs your workers can run at once, every worker is busy.

**The examples below** count the running jobs, show a job whose worker stopped, and print a status
line for a queue.
