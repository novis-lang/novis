Sends one text message to the server on a WebSocket connection your program opened.

The server receives the whole string as one message, however long it is. `send()` returns nothing.
You may send a `tainted` value, for example a message you received earlier. To send binary data,
use `sendBytes()`.

`send()` waits until the message is written, but not forever. If the server stops reading and the
message cannot be written in time, `send()` throws a `TimeoutError`. If the connection breaks, it
throws an `IOError`. After `close()`, `send()` throws a `LogicError`, because nobody receives the
message.

**The examples below** send two messages, show the error after a close, and send one JSON request
for each product a program wants prices for.
