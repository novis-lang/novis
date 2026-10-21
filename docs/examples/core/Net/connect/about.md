Opens a TCP connection to a server, so your program can send and receive bytes over it.

You give the host, the port and the longest time the connection may take to open. The result is a
`Core\Net\Stream`. You call `write` and `read` on it, and `close` when you are done. If you do not
close it, it closes when the request ends.

Your program may connect to any public host with no `nvs.toml`. A `connect` list in the
`[capabilities.net]` block allows only the hosts it names. A host that is not allowed throws a
`RuntimeError` before it is looked up. A server that is not running throws an `IOError`. A server
that does not answer in time throws a `TimeoutError`.

**Good to know:** addresses inside your own network, such as `10.0.0.5`, need a second setting,
`internal`. It takes addresses and ranges such as `10.0.0.0/8`. `127.0.0.1` is allowed from the
command line and in development mode. A server in production needs it in `internal` too.
