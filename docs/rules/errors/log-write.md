```
Core\Log::write(Log\Level $level, string $message, array<string, mixed> $fields = []): void
```

Application code and `rule:errors/handler-script` call this; it is a thin binding over the **same
native record-and-write helper** `rule:errors/engine-floor` calls directly when it has no script to
run at all. One implementation, two callers.

What the two share is the **record** (`rule:errors/diagnostic-record`), not the bytes. JSON Lines is
the log target's default rendering of it — one JSON object per line carrying `ts`, `level`,
`message`, `request_id`, `trace_id` and `span_id` when a trace is active, and `fields`. `[log]
format = "text"` renders the same record for a human, and keeps that target unforgeable through
`rule:errors/record-transformations` rather than through concatenation.

JSON was chosen over `logfmt` because an arbitrary error message or a multi-line stack trace needs
escaping that is correct on the first and only attempt at the floor, and JSON's escaping is a
solved, mechanical problem where `logfmt`'s quoting of embedded quotes, newlines and spaces is not.
