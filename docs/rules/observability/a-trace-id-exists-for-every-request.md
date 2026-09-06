A trace id is drawn for every request when its context is built, whatever the sampling decision.
Sampling (`rule:observability/sampling-is-head-based`) governs whether a trace is *exported*, never
whether an id is generated — and that is what lets one id be Novis's only request identifier.
`Core\Server::traceId()` reads it, every `[log]` record and every error rendering carries it as
`request_id` (`rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`), and it is
emitted on the response so a proxy can log it with one `log_format` line.

There is deliberately **no second identifier and no inbound `X-Request-ID`**. Two identifiers for
one request is two things that can disagree, and a subsystem minting an id of its own where a
record is written would produce a line no backend could join to anything. The id is generated where
the request's state lives, eagerly, rather than by whichever subsystem asks first.

**A trace id never becomes a metric label.** It is unbounded by construction, and
`rule:security/metric-label-refuses-tainted` would refuse it anyway.
