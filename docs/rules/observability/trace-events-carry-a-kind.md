Every trace event carries a `kind` tag, and the tag is one of exactly five: `call`, `gc`, `spawn`,
`query`, `http`. A `call` event is `rule:testing/debug-probes`'s probe pair unchanged — callee,
arguments, entry and exit timestamp, checked-return status, result. The other kinds are emitted from
routines of their own, not from the per-statement or per-call probe: a `gc` from the collector's run
routine (`rule:observability/gc-pause-is-its-own-event`), a `spawn` from each routine that starts a
child — the three isolate ones and `Core\Process::spawn`
(`rule:observability/spawn-is-its-own-event`), a `query` from inside `Core\Db`'s own
statement routine, and an `http` from `Core\Http\Client`'s transport, filed once per call whatever
its attempt count.

A `query` carries the duration, the driver, the connection name, the truncated SQL text, the rows
returned and the rows affected — and **never a bound parameter value**, because a trace is a
`secret` sink (`rule:security/secret-qualifier`). An `http` event answers the same question about an
outbound call: the method, the scheme, host and port, the path without its query, the status, the
attempt and hop counts, the address connected to, and where the time went — and never a query
string, a header value or a body, for the same reason. Every one of those kinds sits in a routine
that is already rare and already slow, so tracing costs nothing on the hot path the probes measure,
and nothing at all when the flag is off.

The tag is what lets one stream serve every consumer: the timeline export, the profiler's
attribution, and production telemetry all read these kinds and add no instrumentation of their
own.
