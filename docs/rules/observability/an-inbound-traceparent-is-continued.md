An inbound W3C `traceparent` header continues the trace: its trace id and parent span id are
adopted, and its sampled flag is honoured. A request that arrived without one is a new trace rather
than no trace (`rule:observability/a-trace-id-exists-for-every-request`).

A header the process cannot read — malformed, or two `traceparent` lines on one request, for which
the W3C format has no combining rule — **starts a new trace rather than throwing**. It arrived from
outside, it is `tainted`, and refusing a request over a bad tracing header would turn an
observability feature into an availability one.

An inbound trace that is already sampled is always continued, because a partially recorded
distributed trace is worse than none; that half of the rule is
`rule:observability/sampling-is-head-based`'s. The other direction is
`rule:observability/an-outbound-call-propagates-traceparent`.
