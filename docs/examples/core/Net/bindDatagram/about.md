Opens a UDP socket on an address and a port. UDP sends short messages called datagrams. There is
no connection: each message is sent on its own, to an address you name when you send it.

The result is a `Core\Net\Datagram`. Its `send` method sends one message, and its `receive` method
waits for the next one. Port `0` lets the operating system pick a free port, and `port` tells you
which one it picked.

The address must be an IP address, such as `127.0.0.1` or `0.0.0.0`, and never a hostname. The exact
address and port must be allowed under `listen` in the `[capabilities.net]` block of `nvs.toml`.
Otherwise the call throws a `RuntimeError`.

**Good to know:** `listen` only allows the socket to open. To send a message, the host you send to
must also be allowed under `connect`.
