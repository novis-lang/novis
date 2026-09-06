A graceful shutdown and a control-socket reload close connections with a defined code after a drain
period, so a client's reconnect logic sees a clean close rather than a reset. There is one mechanism
behind both spellings, because there is one thing a stopping process and a reloading one both need.

The drain period starts when a connection **first sees** the drain, not when the drain began, so a
connection is given the whole period however late it was accepted.

The close is the connection's *own*, taken at its next wait rather than reached in from the accept
loop. The socket belongs to the isolate, so closing it from outside would be writing to a descriptor
another task is parked on — and a dropped descriptor gives the peer a reset, which is exactly what a
client cannot tell apart from a network failure.
