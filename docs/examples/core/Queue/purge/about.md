Deletes many jobs of one queue at once, and returns how many it deleted.

Without options, `Core\Queue::purge` deletes the jobs that succeeded or were cancelled. The `state` option deletes other jobs: `State::Dead` deletes the jobs in the dead-letter table (the table of jobs that failed too often), and `State::Pending` deletes jobs that have not run yet. Jobs that a worker is running are never deleted. The `tag` option deletes only the jobs with that tag. The `before` option deletes only the jobs added before that time.

One call deletes at most `limit` rows, and the default is 1000. A short delete keeps the table free for new jobs. To delete more, call `purge` again until it returns `0`.

**Good to know:** the program needs the `queue.purge` capability for the queue.

**The examples below** delete cancelled jobs, delete the waiting jobs of one tag, and clean up a queue in batches.
