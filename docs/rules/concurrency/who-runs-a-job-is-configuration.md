Whether jobs run inside the server process or in a worker of their own is configuration, not a
different mechanism. `[queue] workers` is a count per *instance* and running workers in-process is the
default shape; `workers = 0` makes an instance enqueue-only, which is how a deployment separates the
machines that accept requests from the ones that drain the queue.

Both spellings drive the identical isolate (`rule:concurrency/a-job-runs-as-a-root-isolate`) over the
identical claim statement (`rule:concurrency/claiming-is-one-statement`), so moving work between them
is an operational decision and never a behavioural one. There is nothing to install beside the runtime
and no supervisor to keep alive.
