Returns the WebSocket connection your script is running for.

When a request calls `Core\Socket::upgrade`, the server starts a new script for the connection. That
script calls `current()` first, and then talks to the client with `receive()` and `send()` on the
connection it got. There is one connection per script, and every call returns an object for that
same connection.

Only a script started for a connection has one. In an ordinary request, a command line program or
a `spawn script` child, `current()` throws a `LogicError`.

**The examples below** send a welcome message, show the error outside a connection, and hand the
connection to a class that handles the chat.
