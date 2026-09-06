The JSON-Lines record `rule:errors/log-write` defines gains `trace_id` and `span_id` whenever a
trace is active. Inside a sampled request both are present; outside one both are **omitted rather
than written as empty strings**, and that omitted-not-empty rule applies to the whole envelope —
a producer with no request to name does not invent one.

`request_id` on the same record is the trace id, because
`rule:observability/a-trace-id-exists-for-every-request` makes that the only identifier there is.
It repeats in `trace_id` for a sampled trace on purpose: a log pipeline correlating by request and a
tracing backend correlating by trace each read their own key, and neither has to know the other's
rule. The id a record carries is the one the server's door decided from the inbound header
(`rule:observability/an-inbound-traceparent-is-continued`), never a second one drawn where the
record was written.

Two fields added to a shape that already exists, and they are what lets an operator jump from a
log line to the trace that produced it — the single highest-value thing an observability stack does.
