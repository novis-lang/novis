The trace id is the request identifier, and there is no second one. A trace id exists for every request because W3C TraceContext generates one regardless of the sampling decision — sampling governs only whether the trace is *exported* — so the id needed to join a log line to an error page to a proxy log entry is already present on every request, with `[trace] sample` at zero included.

`Core\Server::traceId()` reads it, every log record and every error rendering carries it (`rule:errors/log-fields`), and it is emitted on the response so a proxy can log it with one `log_format` line. An inbound `traceparent` is continued; a missing one is generated.

**An inbound `X-Request-ID` is ignored** and appears nowhere. Honouring it would need a validation rule against log injection for a fact Novis already has, and two identifiers for one fact is how people grep the wrong one.
