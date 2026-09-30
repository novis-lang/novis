How many queued jobs this instance works on at the same time.

A deployment often runs the same program on several instances. This setting makes them different:
some instances work on the queue, and some only add jobs to it. With `0`, the instance still adds
jobs, and another instance runs them. This way a web server can give its slow work to other
machines.

**In plain words:** `0` means "I add the work, somebody else does it". To turn the queue off, leave
out the `[queue]` block.

A worker with no job checks the queue once every second. A job that this instance adds starts at
once, because `Core\Queue::push` wakes the workers when the job is saved. A job that another
instance adds can start up to one second late. The same is true for a job with a `runAt` time and
for a job that runs again after an error.

A program cannot change this setting. The server applies a new value while it runs, and starts or
stops workers. A worker that stops first puts its current job back in the queue.

The example prints how many workers this instance runs. Then it tries to change the count, and
`Core\Config::set` returns `false`.
