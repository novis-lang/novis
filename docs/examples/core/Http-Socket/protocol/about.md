Returns the subprotocol the server chose when the WebSocket connection was opened, or `null`.

A subprotocol is a name for the message format both sides use, for example `chat.v2`. Your
program offers one or more names with the `protocols` option of `Core\Http\Client::openSocket()`.
The server chooses one of them, or none. `protocol()` returns the name it chose. It returns `null`
when your program offered no names or the server chose none.

If the server chooses a name your program did not offer, `openSocket()` throws an error. So a name
that `protocol()` returns is always one of the names you offered. The name stays readable after
`close()`.

**The examples below** read the chosen name, handle a server that chose none, and pick a message
format from the name.
