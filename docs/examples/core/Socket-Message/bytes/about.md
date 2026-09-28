Returns the data of a message that the client sent as binary.

A WebSocket client can send two kinds of message: text and binary. `bytes()` returns the data of a
binary message as `bytes`, exactly as the client sent it. For a text message it returns `null`, and
`text()` returns the text instead. It also returns `null` for a message that came from a topic.

The data is `tainted`, because it comes from the client and a client can send anything. A binary
message can be empty. Then `bytes()` returns empty `bytes`, not `null`.

**The examples below** read the size of a binary message, tell a text message from a binary
message, and receive a file that the client sends in several pieces.
