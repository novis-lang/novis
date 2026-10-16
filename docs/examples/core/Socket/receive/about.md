Waits for the next message on the WebSocket connection your script is running for, and returns it.

A message comes from one of two places. The client sent it, or another part of your application
published it to a topic this connection subscribed to. Call `topic()` on the message to tell them
apart: it is `null` for a message from the client. `receive()` returns `null` when the client closes
the connection, so a connection script is one loop that stops at `null`.

What the client sent is `tainted`, because it is untrusted input. If the connection breaks,
`receive()` throws a `RuntimeError`.

**Good to know:** a connection that falls too far behind its topics is closed, and then `receive()`
returns `null`.

**The examples below** read every message, tell text from binary messages, and run a chat
connection that reads from its client and from a room with one loop.
