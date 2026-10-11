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
instead, there being no isolate for a wait to park in, and a request parked on one would never end.

**Both event-stream doors return one type, `Core\Sse`.** A helper that takes a `Core\Sse` and writes
events is written once and called from either side, the shape `Core\Socket::current` has for code
that runs inside a connection. A member one door cannot serve throws there rather than being absent,
and the member each door is missing is the hand-over it reports
(`rule:concurrency/two-doors-one-isolate`).

**Resumption is the client's `Last-Event-ID`, and the runtime keeps no replay buffer.** A connection
isolate shares nothing with the request that opened it (`rule:security/isolate-shares-nothing`), so
the handler reads the header and passes it through `args:`; the request-scoped door reads it
directly. Replay needs the application's own event log, and the id is the key into it.

**A request-scoped event stream is reconnected by `EventSource` when the request ends.** That door is
for `fetch`-based readers and for progress views that close themselves, and a handler that wants the
client to stop answers a non-`200` on the next request.

**Both doors open with `Cache-Control: no-cache, no-transform` and `X-Accel-Buffering: no`.** A
production deployment is a proxied origin, and nginx buffers a proxied response by default, which
holds every event until the stream ends. The second header is an instruction *to* a proxy, not a
proxy behaviour this server implements, so
`rule:http-server/two-deployments-and-nothing-a-proxy-owns` is untouched by it.

Both spellings are built. `Core\Response::stream` and `Core\Sse::stream` write a body that ends with
the response, from inside the request isolate; `Core\Sse::upgrade` opens the connection. Both write
through one cell between the writing isolate and the connection that frames the body, so the two
spellings are one model with two hand-overs rather than two mechanisms.

**One chunk is in flight, and the writer parks until the connection has taken it.** Backpressure is
structural: a program that writes faster than the peer reads is slowed by the peer, nothing
accumulates in between, and there is no queue depth to choose. A writer that could park forever is
the one failure this shape has, so the wait is bounded by the connection's send timeout
(`rule:concurrency/connection-bounds-are-finite`), and a consumer that has gone away closes its
writer at once. A whole body keeps its exact length and its `Content-Length`; a streaming body
reports none and is sent chunked.

Choosing by protocol instead of by lifetime is the mistake the line exists to prevent. Server-sent
events over a request that ends is a streaming response and not a connection; a WebSocket is never
anything but a connection.

- **Memory:** an open event stream holds one wake pair and one chunk in flight, plus the framing
  buffer and subscriber queue a connection already pays for, and opening one draws once from the
  random number generator. That is O(in-flight), never O(events sent), and a response that does not
  stream pays nothing.

**Declined:**
- **A narrower writer type for the request-scoped door.** It would make every member available on
  the type that declares it, and remove the reason one type exists: a helper written once and called
  from both doors.
- **A `send`/`sendJson` split**, on `Core\Socket::send`/`sendBytes`'s model. It doubles the surface
  for a choice the caller never has to make: `send` writes a `string` raw and JSON-encodes anything
  else.
- **The connection door alone, with a writing path of its own.** The event stream's `send` would
  reach the wire through a path the request-scoped spelling could not use, and the language would
  carry two mechanisms for one model.
- **A queue between the writer and the connection.** A queue needs a depth somebody chooses, and
  what it holds grows with how far the peer falls behind; one chunk in flight holds one chunk.
- **A replay buffer in the runtime.** A bounded buffer answers some resumptions and silently not
  others, with the boundary decided by how fast the client reconnected.
