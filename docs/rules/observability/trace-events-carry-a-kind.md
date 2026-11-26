Every trace event carries a `kind` tag, and the tag is one of exactly ten: `call`, `gc`, `spawn`,
`query`, `http`, `cache`, `session`, `file`, `mail` and `queue`. The last two are built by goal
`deeper-debug-records`. A `call` event is `rule:testing/debug-probes`'s
probe pair unchanged — callee, arguments, entry and exit timestamp, checked-return status, result.
The other kinds are emitted from routines of their own, not from the per-statement or per-call probe: a `gc` from the collector's run
routine (`rule:observability/gc-pause-is-its-own-event`), a `spawn` from each routine that starts a
child — the three isolate ones and `Core\Process::spawn`
(`rule:observability/spawn-is-its-own-event`), a `query` from inside `Core\Db`'s own
statement routine, and an `http` from `Core\Http\Client`'s transport, filed once per call whatever
its attempt count. A `cache`, a `session` and a `file` event come from the member a program called:
a `Core\Cache\Store` read or write, a `Core\Session` read or write, and a `Core\IO` member that opens
a file, each on `nvs_stdlib::registry::RECORDED_SITE_MEMBERS`, filed once per call with its duration,
the member's name and its target, and with ` failed` after the name when the call threw. A `mail`
event comes from `Core\Mail::send` and a `queue` event from `Core\Queue::push`, each filed once per
call where the call returns or throws.

A `query` carries the duration, the driver, the connection name, the truncated SQL text, the rows
returned and the rows affected — and **never a bound parameter value**, because a trace is a
`secret` sink (`rule:security/secret-qualifier`). An `http` event answers the same question about an
outbound call: the method, the scheme, host and port, the path without its query, the status, the
attempt and hop counts, the address connected to, and where the time went — and never a query
string, a header value or a body, for the same reason. A `cache`, `session` or `file` event's
target is the key or the file path, and never the value read or written; a `getSecret`,
`putSecret` or `setSecret` call has no target. A `mail` event carries the number of recipients and
whether the send succeeded, and never an address, a subject or a body, because each is a person's
data. A `queue` event carries the queue's name, the job's script and its delay, and never the job's
arguments, which are the program's data. Each event's outcome fields are named in
`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`. Every one of those kinds sits in a routine
that is already rare and already slow, so tracing costs nothing on the hot path the probes measure,
and nothing at all when the flag is off.

The tag is what lets one stream serve every consumer: the timeline export, the profiler's
attribution, and production telemetry all read these kinds and add no instrumentation of their
own. The price is that every exporter handles every kind, so each new kind is surface in all of them
at once, and a kind is added only for a routine whose cost the existing ones cannot show. The
`cache`, `session` and `file` kinds are added for the editor's recorded lens
(`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`), which counts them by line.
They become no span of their own (`rule:observability/four-kinds-become-a-span`): a cache read is
cheap beside a span's export, and a request that reads a key in a loop would fill its span ceiling.
The `mail` and `queue` kinds are added because a request page cannot otherwise show that a request
sent mail or pushed a job, which are the two effects it leaves outside its own response. They become
no span either: each is one call, and its time is already inside the request's span.

**Declined:**
- **A `mail` event with its address, its subject or its body.** Each is a person's data, and the
  index keeps a record for days, readable by anyone on the viewer.
- **A `queue` event with the job's arguments.** They are the program's data, and they may carry a
  credential the job needs.
- **A session id on an event or a request, hashed or not.** It links one person's requests, so the
  index would become a record of who did what.
- **A kind for a coroutine's suspend and resume.** It would sit in the same cheap, already-slow place
  as `gc` and `spawn`, but no debugging need for the scheduling gap between the two has been named.
- **Starting or stopping tracing from outside a running process.** A channel that reaches into a live
  process and reads call arguments and timing is a new trust boundary, not an export format, and it
  would need a threat model of its own before it could exist.
- **An allocation or heap-snapshot timeline.** No probe records an allocation, and adding one is a
  larger design than a further kind on this stream.
