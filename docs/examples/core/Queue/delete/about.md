Deletes the database row of one background job.

You pass the id that `Core\Queue::push` returned. `delete` removes the row of the job and returns `true`. It works for a waiting job, a finished job, a cancelled job and a job in the dead-letter table (the table of jobs that failed too often). It returns `false` when a worker is running the job, or when the row is already gone. After a `delete`, `status` throws an error for that job.

`cancel` only stops a job and keeps its row. Use `delete` when the data in the row must be gone, for example when a user deletes their account.

**Good to know:** the program needs the `queue.purge` capability for the queue of the job. A server has no such capability until the person who runs it adds one.

**The examples below** delete a waiting job, delete a cancelled job, and remove the jobs of a user who deleted their account.
