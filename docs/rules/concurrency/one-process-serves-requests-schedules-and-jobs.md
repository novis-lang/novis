`nvs serve` is one process running three things — the accept loop, the `[[schedule]]` ticker and the
queue's workers — on the scheduler it already turns, and the drain stops all three. A deployment
installs one unit and supervises one process; there is nothing to run beside it, which is the whole of
what `rule:concurrency/no-broker-and-no-driver-interface`'s "nothing to install beside the runtime and
no supervisor to keep alive" is worth.

The three are armed the same way and are not variants of one mechanism
(`rule:concurrency/queued-work-is-not-scheduled-work` keeps the middle two apart). A tree writing no
`[queue]` block arms no worker and a tree writing no `[[schedule]]` spawns no ticker, so each costs a
boot-time read and no task at all when it is not configured.

**The drain is what stops a worker, and it is not merely tidy.** A drain means *stop taking new work*,
and claiming a job is taking new work — so a claimed job runs to completion exactly as an accepted
request and an in-flight fire do, and nothing new is claimed after it begins. It is also what keeps
the process able to end: the server's loop runs while anything is parked and a worker waiting for work
is always parked, so a worker that ignored the drain would be a server nothing but killing the process
could stop — which is the graceful shutdown a terminating signal, a service manager and a test harness
each ask of one. The drain wakes a worker out of that wait, so a shutdown does not wait for it to run
out (`rule:concurrency/a-push-wakes-an-idle-worker`).

**The queue needs no lease and a `fleet` schedule does.** A worker asks the database for a row and
gets one or does not (`rule:concurrency/claiming-is-one-statement`), so a fleet of instances each
running their own workers is the intended deployment rather than a hazard; a scheduled entry has to
ask whether another host is already firing it. The queue is the subsystem that needed no coordinator,
and one process is where that pays.
