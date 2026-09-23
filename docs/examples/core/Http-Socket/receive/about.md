Waits for the next message from the server on a WebSocket connection your program opened, and
returns it.

The result is a `Core\Socket\Message`. A text message has its content in `text()`, and a binary
message has it in `bytes()`. The other of the two is `null`. Both are `tainted`, because the
content came from outside your program. `receive()` returns `null` when the conversation is over:
the server closed the connection, or your program called `close()`.

`receive()` waits, but not forever. If the server sends nothing for longer than the `idle` time,
it throws a `TimeoutError`. A message larger than `maxMessage` closes the connection and throws a
`RuntimeError`.

**The examples below** read every message until the end, tell text and binary messages apart, and
answer each request a server sends.
