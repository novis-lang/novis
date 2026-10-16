Ends a WebSocket connection that your program opened.

`close()` sends a close message to the server and waits a short time for the server to answer.
Then the connection is gone. After a close, `receive()` returns `null`, and `send()` throws a
`LogicError`. Calling `close()` a second time does nothing.

You can give a close code and a reason. The code is a number that says why the connection ended.
If you leave it out, it is `1000`, which means a normal ending. The reason is a short text the
server may write to its log. A server that never answers the close does not cause an error.

**Good to know:** a connection is also closed when the request or task that opened it ends. Call
`close()` when you are finished earlier, so the server stops sending.
