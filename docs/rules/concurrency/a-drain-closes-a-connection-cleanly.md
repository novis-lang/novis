A graceful shutdown and a control-socket reload close connections with a defined code after a drain
period, so a client's reconnect logic sees a clean close rather than a reset. There is one mechanism
behind both spellings, because there is one thing a stopping process and a reloading one both need.

The drain period is `[server] drain_timeout`, `"30s"` with nothing configured and never unbounded
(`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`). It starts when a connection **first
sees** the drain, not when the drain began, so a connection is given the whole period however late
it was accepted.

A connection is made to see it: **beginning the drain wakes every parked wait**, rather than leaving
a connection to notice at its own idle timer — which would compose the drain period with
`keepalive_timeout` and make a stop take the longest wait a deployment configured before the period
even started. A wake is not a close; it makes the task runnable, and the close below is still the
one it takes itself.

The close is the connection's *own*, taken at its next wait rather than reached in from the accept
loop. The socket belongs to the isolate, so closing it from outside would be writing to a descriptor
another task is parked on — and a dropped descriptor gives the peer a reset, which is exactly what a
client cannot tell apart from a network failure.
