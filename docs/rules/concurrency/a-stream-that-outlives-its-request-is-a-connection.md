The line between a streaming response and a connection is **what the stream outlives**, and it is
worth stating sharply because it is the one place two spellings could appear for one job.

A stream that ends when the response ends is a *streaming response*: progress for one request, a
large export, a chunked file. It stays in the request isolate and is bounded by the request's budget.

A stream that outlives its request is a *connection isolate*: notifications, a live dashboard,
anything a client keeps open across page lifetimes. `Core\Sse::upgrade` opens one, and it is the same
isolate a WebSocket gets — a root, its own arena, its own budget, and `send`.

**What an event stream has no `receive` for is a peer, not a wait.** Its client cannot send on the
stream, so there is no second source of frames; topics still arrive, and `receive()` on a
connection-scoped event stream waits on its subscriptions alone. On the request-scoped door it throws
instead, there being no isolate for a wait to park in.

Both spellings are built. `Core\Response::stream` and `Core\Sse::stream` write a body that ends with
the response, from inside the request isolate; `Core\Sse::upgrade` opens the connection. Neither
existed when this line was first drawn, and the line itself is what did not change when they landed.

Choosing by protocol instead of by lifetime is the mistake the line exists to prevent. Server-sent
events over a request that ends is a streaming response and not a connection; a WebSocket is never
anything but a connection.
