Turns a request into a WebSocket connection. A WebSocket connection stays open, so the client and
the server can send each other messages at any time.

You name what the connection runs: the path of a file, or a static method written `Chat::run(...)`.
The connection starts when the request ends. It runs in its own isolate (a separate copy of the
program that shares no memory with the request). `args:` copies values into it.

The other options are the ones `spawn script` takes. `limits:` sets lower limits, such as
`{ memory: "8M" }`. `grants:` lists the capabilities the connection may use, and it gets only the
ones the request also has. `on:` can only be `"here"`, so `on: "worker"` does not compile.

Only a request that asks for a WebSocket can be upgraded. Any other request throws a
`RuntimeError`, and so does a second call in the same request.

**The examples below** open a connection that runs a file, show the error for a normal page
request, and open a chat connection only for a signed-in user.
