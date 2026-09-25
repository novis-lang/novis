Opens a TCP connection to a server, so your program can send and receive bytes over it.

You give the host, the port and the longest time the connection may take to open. The result is a
`Core\Net\Stream`. You call `write` and `read` on it, and `close` when you are done. If you do not
close it, it closes when the request ends. This replaces PHP's `fsockopen` and
`stream_socket_client`.

The host must be allowed in the `[capabilities.net]` block of `nvs.toml`. Otherwise the call throws a
`RuntimeError` before it looks the host up. A server that is not running throws an `IOError`. A server
that does not answer in time throws a `TimeoutError`.

**Good to know:** addresses inside your own network, such as `127.0.0.1` or `10.0.0.5`, need a second
setting, `internal`, even when the host is allowed.
