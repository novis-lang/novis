A trace id is drawn for every request when its context is built, whatever the sampling decision.
Sampling (`rule:observability/sampling-is-head-based`) governs whether a trace is *exported*, never
whether an id is generated, so the id exists with `[trace] sample` at zero as well — and that is what
lets one id be Novis's only request identifier, the one that joins a log line to an error page to a
proxy's log entry. An inbound `traceparent` is continued and a missing one is generated
(`rule:observability/an-inbound-traceparent-is-continued`). `Core\Server::traceId()` reads it, every
`[log]` record and every error rendering carries it as `request_id`
(`rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`, `rule:errors/log-fields`),
and it is emitted on the response so a proxy can log it with one `log_format` line.

There is deliberately **no second identifier, and an inbound `X-Request-ID` is ignored** and appears
nowhere. Two identifiers for one request is two things that can disagree and two things to grep, and
honouring the header would need a validation rule against log injection for a fact Novis already has.
A subsystem minting an id of its own where a record is written would produce a line no backend could
join to anything, so the id is generated where the request's state lives, eagerly, rather than by
whichever subsystem asks first.

**A trace id never becomes a metric label.** It is unbounded by construction, and
`rule:security/metric-label-refuses-tainted` would refuse it anyway.

**Not on disk.** No response header carries the id yet. Everything else here is built: the id is drawn
with every context (`crates/nvs-runtime/src/trace_context.rs:98`) and continued or generated at the
door whatever the sampling (`crates/nvs-server/src/trace.rs:120`).
