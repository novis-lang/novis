A distributed trace is not the per-call trace `rule:testing/debug-probes` produces, and conflating
them would give a trace one span per function call, which no backend can store and no human can
read. So **exactly four things become a span**: the request (or scheduled run) root, a `query`
event (`rule:observability/a-query-is-a-trace-event`), an outbound `Core\Http\Client` call, and a
`spawn` event. A `call`-kind event **never** becomes a span. A `gc` event becomes the pause
histogram in `rule:observability/default-series`, not a span — a collection pause is not a unit of
work in a request's causal graph.

The spans are derived from the same events the timeline already files; there is no second set of
probes for tracing, and export cost stays off the hot path the debug probes deliberately keep cheap.
A retried outbound call is one span carrying an attempt count, not one span per attempt.
