Whether jobs run inside the server process or in a worker of their own is configuration, not a
different mechanism. `[queue] workers` is a count per *instance* and running workers in-process is the
default shape; `workers = 0` makes an instance enqueue-only, which is how a deployment separates the
machines that accept requests from the ones that drain the queue.

**An instance is a process and not a core.** `nvs serve` arms the count once, on the core it arms the
`[[schedule]]` ticker on (`rule:concurrency/one-process-serves-requests-schedules-and-jobs`), so a
thread-per-core server does not multiply an operator's number by its core count — `workers = 4` on a
32-core host is four workers and four connections. A deployment that wants thirty-two writes
thirty-two.

Both spellings drive the identical isolate (`rule:concurrency/a-job-runs-as-a-root-isolate`) over the
identical claim statement (`rule:concurrency/claiming-is-one-statement`), so moving work between them
is an operational decision and never a behavioural one. There is nothing to install beside the runtime
and no supervisor to keep alive.
