How long a worker keeps a claimed job to itself.

Claiming a job takes a lease on it: for this long no other worker can see it. A worker that
finishes is done and the lease never matters. A worker that dies mid-job — the host was restarted,
the process was killed — leaves the lease to run out, and then the job is visible again and somebody
else picks it up. That is the whole recovery story, and nothing has to notice the crash for it to
work.

**In plain words:** how long to wait before assuming the worker that took this job is not coming
back.

Write it comfortably longer than the work takes. A lease shorter than the job means a second worker
starts the same job while the first is still running it, and a lease of nothing is refused when the
deployment starts, because it makes every claimed job visible to everybody at once. The key is the
operator's, and a change reaches the next job claimed rather than the next restart.

The example prints how long a claim is held here and is turned away shortening its own.
