Connects to a server on the same computer through a socket file. This kind of connection is called
a Unix-domain socket. Many local services accept connections this way, such as a database or a
cache. This replaces the `unix://` form of PHP's `stream_socket_client`.

You give the path of the socket file and the longest time the connection may take to open. The
result is a `Core\Net\Stream`, the same as for `Core\Net::connect`. You call `write` and `read` on
it, and `close` when you are done.

The path must be allowed under `local` in the `[capabilities.net]` block of `nvs.toml`. Otherwise
the call throws a `RuntimeError`. When nothing listens at the path, it throws an `IOError`. When the
time runs out, it throws a `TimeoutError`.

**Good to know:** socket files work on Linux and other Unix systems. On Windows, this call throws a
`RuntimeError`.
