Returns the text of a message that the client sent as text.

A WebSocket client can send two kinds of message: text and binary. `text()` returns the text of a
text message, exactly as the client sent it. For a binary message it returns `null`, and `bytes()`
returns the data instead. It also returns `null` for a message that came from a topic.

The text is `tainted`, because it comes from the client and a client can send anything. You can
print it, compare it and send it back. Before you use it where only trusted text is allowed, check it
with `Core\Validate`.

**The examples below** read what the client typed, tell a binary message from a text message, and
read chat commands from the text.
