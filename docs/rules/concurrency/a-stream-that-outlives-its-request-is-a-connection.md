The line between a streaming response and a connection is **what the stream outlives**, and it is
worth stating sharply because it is the one place two spellings could appear for one job.

A stream that ends when the response ends is a *streaming response*: progress for one request, a
large export, a chunked file. It stays in the request isolate and is bounded by the request's budget.

A stream that outlives its request is a *connection isolate*: notifications, a live dashboard,
anything a client keeps open across page lifetimes. `Core\Sse::upgrade` opens one, and it is the same
isolate a WebSocket gets — a root, its own arena, its own budget, `send` and no `receive`.

Choosing by protocol instead of by lifetime is the mistake the line exists to prevent. Server-sent
events over a request that ends is a streaming response and not a connection; a WebSocket is never
anything but a connection.
