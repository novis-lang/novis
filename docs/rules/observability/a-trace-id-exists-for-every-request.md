A trace id is drawn for every request when its context is built, whatever the sampling decision.
Sampling (`rule:observability/sampling-is-head-based`) governs whether a trace is *exported*, never
whether an id is generated, so the id exists with `[trace] sample` at zero as well. An inbound
`traceparent` is continued and a missing one is generated
(`rule:observability/an-inbound-traceparent-is-continued`). `Core\Server::traceId()` reads it, and
by default every response carries it in W3C Trace Context's `traceresponse` header, so a proxy can
log it with one `log_format` line and a tracing backend reads one standard header. `[trace]
response_header = false` removes the header and nothing else
(`rule:observability/the-traceresponse-header-can-be-turned-off`): the id, the logs and the spans
are unchanged.

**The request id is always the server's own.** Every `[log]` record and every error rendering
carries it as `request_id`
(`rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`, `rule:errors/log-fields`).
For a trace this process roots it is the trace id, one number in every place. A continued trace's id
is whatever the client sent, so two requests can carry one; such a request gets a request id drawn
here, and its records carry the trace id as `trace_id` beside it. A client therefore cannot make two
requests share a log key. What it spends: 16 bytes per request in flight.

The header is `00-<trace id>-<the request's span id>-<flags>`, the `traceparent` shape, written
sampled or not; the flag says which, so a backend that holds no trace for a `-00` id knows why. It is
written at the one point every answer of the door passes (`crates/nvs-server/src/serve.rs`'s
`counted`), and it replaces a `traceresponse` the program set for itself, because the request has one
id. An answer the door wrote before any carrier existed — a refusal at the valve, a preflight, a
mount table's own answer — gets a new unsampled root there, and its access line names the same id.
What it spends: one 55-byte header value per response, and none with the header off.

**An inbound `X-Request-ID` is ignored** and appears nowhere — not as the trace id, not on the
response, not on the access line. Honouring the header would need a validation rule against log
injection for a fact Novis already has, and it is a client's choice exactly as a continued trace id
is. A subsystem minting an id of its own where a record is written would produce a line no backend
could join to anything, so both ids are generated where the request's state lives, eagerly, rather
than by whichever subsystem asks first.

**A trace id never becomes a metric label.** It is unbounded by construction, and
`rule:security/metric-label-refuses-tainted` would refuse it anyway.

**Declined:**
- **An `X-Request-ID` response header beside `traceresponse`.** It is a second spelling of an id the
  response already carries.
- **The trace id as the request id for a continued trace.** It is the client's to choose, so a client
  could give many requests one id, and an application that keys on it would mix them up.
- **Continuing an inbound trace only from a trusted proxy.** It needs a list of proxies for a
  tracing header, and the server-drawn request id already keeps the log key the server's.
