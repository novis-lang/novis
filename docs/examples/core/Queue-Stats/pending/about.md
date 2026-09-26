Counts the jobs of one queue that wait for a worker.

`Core\Queue::stats` returns a `Core\Queue\Stats`, and `pending()` is one of its numbers. A job is
pending from the moment `Core\Queue::push` saves it until a worker starts it. A job that a worker runs
now is counted by `claimed()`, not by `pending()`.

Some pending jobs cannot start yet. A job pushed with a `runAt` time in the future is pending. A job
that failed and waits before its next attempt is pending too. So `pending()` can be larger than the
work your workers can start right now.

If `pending()` grows and does not go down, your workers cannot keep up with the queue.

**The examples below** count the waiting jobs, show jobs that wait for a later time, and warn when too
many jobs wait in a queue.
