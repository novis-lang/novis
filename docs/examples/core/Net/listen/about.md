Starts a TCP server: it opens a socket on an address and a port, and waits for clients to connect.

The result is a `Core\Net\Listener`. Its `accept` method waits for the next client and returns a
`Core\Net\Stream` for that client. Port `0` lets the operating system pick a free port, and `port`
tells you which one it picked. This replaces PHP's `stream_socket_server`.

The address must be an IP address, such as `127.0.0.1` or `0.0.0.0`, and never a hostname. The exact
address and port must be allowed under `listen` in the `[capabilities.net]` block of `nvs.toml`.
Otherwise the call throws a `RuntimeError`. A port that another program already uses throws an
`IOError`.

**Good to know:** `listen` does not wait. It returns as soon as the socket is open, and `accept` is
the method that waits.
