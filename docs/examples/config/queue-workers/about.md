How many queued jobs this instance works on at the same time.

A deployment often runs the same program on several instances. This setting makes them different:
some instances work on the queue, and some only add jobs to it. With `0`, the instance still adds
jobs, and another instance runs them. This way a web server can give its slow work to other
machines.

**In plain words:** `0` means "I add the work, somebody else does it". To turn the queue off, leave
out the `[queue]` block.

A program cannot change this setting. The server applies a new value while it runs, and starts or
stops workers. A worker that stops first puts its current job back in the queue.

The example prints how many workers this instance runs. Then it tries to change the count, and
`Core\Config::set` returns `false`.
