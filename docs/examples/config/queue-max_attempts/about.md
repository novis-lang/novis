How many times a failing job is tried before it is set aside.

Work fails for reasons that pass — a database that was restarting, a service briefly unreachable —
and work fails for reasons that never will, and this key is where the deployment draws the line
between them. A job that fails is retried with a growing gap between tries until it has had this
many attempts; then it moves to the dead-letter table and stops. Nothing is retried forever and
nothing is dropped: the job is still there to read, fix and re-queue.

**In plain words:** how many tries before this one is a person's problem rather than the machine's.

There is no spelling for unbounded, and none for zero either — a job that may never be attempted is
dead-lettered by the enqueue that created it, so a deployment asking for that is refused when it
starts. The key is the operator's, but a change to it reaches the next job rather than the next
restart, because it is read out of the standing configuration each time one is claimed.

The example prints how many attempts a job gets here and is turned away giving its own work more.
