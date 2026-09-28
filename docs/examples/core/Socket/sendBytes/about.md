Sends one binary message to the client on the WebSocket connection your script is running for.

It works like `send()`, but the message is bytes instead of text. The bytes are sent as they are, and
nothing checks them, so this is the method for images, compressed data and packed numbers. The client
receives them as one binary message. `sendBytes()` returns nothing.

`sendBytes()` waits until the message is written, but not forever. If the client stops reading and
the message cannot be written in time, it throws a `RuntimeError`. Outside a connection, it throws a
`LogicError`.

**The examples below** send a few bytes, send a binary message back to the client, and send sensor
readings packed as bytes.
