Sends bytes to the other side of a TCP connection. This replaces PHP's `fwrite` on a socket.

`write` returns how many bytes it sent. This can be fewer than you gave it, when the connection
cannot take more right now. To send a whole message, call `write` again with the bytes that are
left, until nothing is left.

If the connection cannot take any bytes within `$within`, `write` throws a `TimeoutError`. This
happens when the other side stops reading. If the other side has closed the connection, `write`
throws an `IOError`.

**The examples below** show the count that `write` returns, a write that times out, and a function
that sends a whole message.
