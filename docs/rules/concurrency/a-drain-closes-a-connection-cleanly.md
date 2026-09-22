A graceful shutdown and a control-socket reload close connections with a defined code after a drain
period, so a client's reconnect logic sees a clean close rather than a reset. There is one mechanism
behind both spellings, because there is one thing a stopping process and a reloading one both need.

The drain period is `[server] drain_timeout`, `"30s"` with nothing configured and never unbounded
(`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`). It bounds a connection that is
**working** — a request whose head, body or response is moving, or a WebSocket a program is speaking
on — and it starts when that connection **first sees** the drain, not when the drain began, so a
connection is given the whole period however late it was accepted.

A connection that is **idle between requests closes the moment it sees the drain**, not at the
period's end. It has nothing to finish, an HTTP/1.1 client already accepts a server closing an idle
kept-alive connection at any moment, and a stop that waited the period out for it would take the
whole of `drain_timeout` for a browser's one open connection. The idle wait after a response that
was moving when the drain began ends the same way, so a stop is bounded by the requests in flight
and never by the connections that happen to be open.

A connection is made to see it: **beginning the drain wakes every parked wait**, rather than leaving
a connection to notice at its own idle timer — which would compose the drain period with
`keepalive_timeout` and make a stop take the longest wait a deployment configured before the period
even started. A wake is not a close; it makes the task runnable, and the close below is still the
one it takes itself.

The close is the connection's *own*, taken at its next wait rather than reached in from the accept
loop. The socket belongs to the isolate, so closing it from outside would be writing to a descriptor
another task is parked on — and a dropped descriptor gives the peer a reset, which is exactly what a
client cannot tell apart from a network failure.
