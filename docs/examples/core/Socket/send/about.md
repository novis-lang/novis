Sends one text message to the client on the WebSocket connection your script is running for.

A connection script gets the connection with `Core\Socket::current()` and calls `send()` on it. The
client receives the whole string as one message. `send()` returns nothing. You may send a `tainted`
string, for example a message the client sent you. To send binary data, use `sendBytes()`.

`send()` waits until the message is written, but not forever. If the client stops reading and the
message cannot be written in time, `send()` throws a `RuntimeError`. Outside a connection, it throws
a `LogicError`.

**The examples below** answer each message, handle a client that stopped reading, and forward the
messages of a chat room to the client.
