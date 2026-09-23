Sends one binary message to the server on a WebSocket connection your program opened.

A WebSocket message is either text or binary. `send()` sends text, and `sendBytes()` sends a
`bytes` value as binary. The server receives all the bytes as one message. `sendBytes()` returns
nothing. You may send `tainted` bytes, for example data you received earlier.

`sendBytes()` fails in the same ways as `send()`. It throws a `TimeoutError` if the server stops
reading and the message cannot be written in time, and an `IOError` if the connection breaks. After
`close()`, it throws a `LogicError`.

**The examples below** send four bytes, show how text and binary messages differ, and send sensor
readings packed into six bytes each.
