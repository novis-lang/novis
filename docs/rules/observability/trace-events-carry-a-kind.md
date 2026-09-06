Every trace event carries a `kind` tag, and the tag is one of exactly four: `call`, `gc`, `spawn`,
`query`. A `call` event is `rule:testing/debug-probes`'s probe pair unchanged — callee, arguments,
entry and exit timestamp, checked-return status, result. The three other kinds are emitted from
routines of their own, not from the per-statement or per-call probe: a `gc` from the collector's run
routine (`rule:observability/gc-pause-is-its-own-event`), a `spawn` from the three isolate-spawn
routines (`rule:observability/spawn-is-its-own-event`), and a `query` from inside `Core\Db`'s own
statement routine.

A `query` carries the duration, the driver, the connection name, the truncated SQL text, the rows
returned and the rows affected — and **never a bound parameter value**, because a trace is a
`secret` sink (`rule:security/secret-qualifier`). Every one of the three sits in a routine that is
already rare and already slow, so tracing costs nothing on the hot path the probes measure, and
nothing at all when the flag is off.

The tag is what lets one stream serve every consumer: the timeline export, the profiler's
attribution, and production telemetry all read these four kinds and add no instrumentation of their
own.
