A distributed trace is not the per-call trace `rule:testing/debug-probes` produces, and conflating
them would give a trace one span per function call, which no backend can store and no human can
read. So **exactly four things become a span**: the request (or scheduled run) root, a `query`
event (`rule:observability/a-query-is-a-trace-event`), an outbound `Core\Http\Client` call, and a
`spawn` event. A `call`-kind event **never** becomes a span. A `gc` event becomes the pause
histogram in `rule:observability/default-series`, not a span — a collection pause is not a unit of
work in a request's causal graph.

The spans are derived from the same events the timeline already files: production telemetry reads
the four event kinds the trace records and adds no instrumentation of its own. A retried outbound call
is one span carrying an attempt count, not one span per attempt. A span's start and end are its
event's, read on the one process clock every event is timed on. The root's are read before and after
the run, so its window contains every other span's.

The set is closed because of what one more kind would cost. `rule:testing/debug-probes` keeps the
per-call path to a load and a predicted-not-taken branch, and every other kind was placed off that
path for the same reason (`rule:observability/gc-pause-is-its-own-event`,
`rule:observability/spawn-is-its-own-event`). A `call` span would put export cost back on it.

**Declined:**
- **A span per `call` event**, which would reuse the instrumentation with no filtering. The trace is
  unstorable and unreadable, and export cost lands on the per-call path.
- **One span per attempt of a retried outbound call.** Each attempt's latency is not yet worth the
  multiplication of spans; the attempt count on the one span carries the fact.
