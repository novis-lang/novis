Opens a WebSocket connection to another server, so your program can send and receive messages while
the connection stays open.

A normal HTTP request gets one response, and then it is finished. A WebSocket stays open. `send`
and `sendBytes` write a message, `receive` returns the next message from the other server, and
`close` ends the connection. When the other server closes it, `receive` returns `null`.

The URL must start with `ws://` or `wss://`. The host must be allowed by `net.connect` in
`[capabilities.net]` in `nvs.toml`, and an address inside your own network throws a `RuntimeError`.
`idle` is how long the connection may stay quiet, `maxDuration` is how long it may stay open, and
`maxMessage` is the largest message in bytes.

**Good to know:** the connection is closed when the request, command or job that opened it ends.

**The examples below** run without a network, so every connection in them fails before it opens.
They show a refused address, the checks on the URL and the options, and a price feed with a
fallback.
