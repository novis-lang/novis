Returns the address of the program that sent a message, as text such as `127.0.0.1` or `::1`. It
is always an address and never a host name.

The result is a plain `string`, so you can give it straight to `Core\Net\Datagram::send` to reply.
`send` still checks the address against the `connect` list in the `[capabilities.net]` block of
`nvs.toml`, the same as any other address. A reply to an address that is not in that list throws a
`RuntimeError`.

Anybody on the network can send a message to your socket. Compare the address with a list of
addresses you trust before you act on a message.
