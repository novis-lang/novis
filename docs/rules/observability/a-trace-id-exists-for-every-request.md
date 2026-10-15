A trace id is drawn for every request when its context is built, whatever the sampling decision.
Sampling (`rule:observability/sampling-is-head-based`) governs whether a trace is *exported*, never
whether an id is generated, so the id exists with `[trace] sample` at zero as well — and that is what
lets one id be Novis's only request identifier, the one that joins a log line to an error page to a
proxy's log entry. An inbound `traceparent` is continued and a missing one is generated
(`rule:observability/an-inbound-traceparent-is-continued`). `Core\Server::traceId()` reads it, every
`[log]` record and every error rendering carries it as `request_id`
(`rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`, `rule:errors/log-fields`),
and every response carries it in W3C Trace Context's `traceresponse` header, so a proxy can log it
with one `log_format` line and a tracing backend reads one standard header.

The header is `00-<trace id>-<the request's span id>-<flags>`, the `traceparent` shape, written
sampled or not; the flag says which, so a backend that holds no trace for a `-00` id knows why. It is
written at the one point every answer of the door passes (`crates/nvs-server/src/serve.rs`'s
`counted`), and it replaces a `traceresponse` the program set for itself, because the request has one
id. An answer the door wrote before any carrier existed — a refusal at the valve, a preflight, a
mount table's own answer — gets a new unsampled root there, and its access line names the same id.
What it spends: one 55-byte header value per response.

There is deliberately **no second identifier, and an inbound `X-Request-ID` is ignored** and appears
nowhere — not as the trace id, not on the response, not on the access line. Two identifiers for one
request is two things that can disagree and two things to grep, and honouring the header would need a
validation rule against log injection for a fact Novis already has. A subsystem minting an id of its
own where a record is written would produce a line no backend could join to anything, so the id is
generated where the request's state lives, eagerly, rather than by whichever subsystem asks first.

**A trace id never becomes a metric label.** It is unbounded by construction, and
`rule:security/metric-label-refuses-tainted` would refuse it anyway.

**Declined:** an `X-Request-ID` response header beside `traceresponse` — a second spelling of the
same id, which is the second identifier this rule rules out.
