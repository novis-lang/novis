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

**A worker's task root is a worker, not the ticker's request.** A schedule fire is a child of the loop
that serves, while there is no request beneath a worker to charge a panic to
(`rule:http-server/containment-does-not-end-at-the-helper`). `nvs run` arms the same workers and
stops them when the script's task exits, because a run with a worker that ignored that would never
end. The two commands differ in that one predicate, which is what keeps moving work between them an
operational decision (`rule:concurrency/who-runs-a-job-is-configuration`).

**The command is still `serve`, and its help names all three subsystems.** The operator's picture of
one process is what stops them looking for a second one. `service` is the service-manager namespace
beside it, and its argv is written into units on disk (`rule:packaging/a-service-is-one-stored-argv`).

- **Memory:** one database connection per worker, on the instance that arms them, held for the life
  of the process. That is per instance and not per request, and it is what the same workers cost under
  `nvs run`.
- **Latency:** workers share the cores the server is already turning, so a busy server and a busy
  queue contend. That is the trade `workers` lets an operator make. `workers = 0` and a dedicated
  `nvs run` host is the deployment that wants them isolated. Nothing on the request path changes: a
  waiting worker is one more parked task, and a parked task costs its stack.

**Declined:**
- **A `nvs queue work` command.** It is a third process shape with its own argv and lifecycle to
  supervise, and it would still leave `[queue] workers` doing nothing under `nvs serve`.
- **A stop flag of the command's own beside the drain.** Two bits would mean the same thing, and the
  second is the one somebody forgets to set: that is the fail-open direction the accept loop already
  refuses.
- **Renaming the command.** A rename would collide with `service` and invalidate the argv stored in
  deployed units. A help text that names the three subsystems buys the same accuracy.
