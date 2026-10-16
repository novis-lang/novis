Starts a server on a socket file, so programs on the same computer can connect to it by its path.
This kind of socket is called a Unix-domain socket, and it opens no network port.

The result is a `Core\Net\Listener`, the same as for `Core\Net::listen`. Its `accept` method waits
for the next client and returns a `Core\Net\Stream` for that client. Clients connect with
`Core\Net::connectLocal`.

The path must be allowed under `local` in the `[capabilities.net]` block of `nvs.toml`. Otherwise
the call throws a `RuntimeError`. A path where a file already exists throws an `IOError`.

**Good to know:** the socket file stays on disk after the server closes. Delete it with
`Core\IO::remove` before you listen at the same path again. Socket files work on Linux and other
Unix systems. On Windows, this call throws a `RuntimeError`.
