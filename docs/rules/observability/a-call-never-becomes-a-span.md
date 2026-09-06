Production telemetry reads the same four event kinds the trace records and adds no instrumentation
of its own — and it is bound by one rule the taxonomy owns: **a `call`-kind event never becomes a
distributed-tracing span.** Exactly four things do: the request or scheduled-run root, a `query`, an
outbound HTTP call, and a `spawn`. A `gc` event becomes a metric, not a span, because a collection
pause is not a unit of work in a request's causal graph.

The set is closed for two reasons. A trace with one span per function call is unstorable by any
backend. And admitting one would put export cost on the per-call path that
`rule:testing/debug-probes` keeps to a load and a predicted-not-taken branch — the cost class every
other kind was placed off of on purpose (`rule:observability/gc-pause-is-its-own-event`,
`rule:observability/spawn-is-its-own-event`).
