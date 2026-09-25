Sends one message over UDP to a host and a port. The message is sent whole, as one datagram, and
the method returns how many bytes it sent. This replaces PHP's `stream_socket_sendto`.

`send` does not wait for an answer and does not check that the message arrived. UDP gives no such
promise. If the other side must answer, it sends its own message back, and you read it with
`receive`.

The host must be allowed under `connect` in the `[capabilities.net]` block of `nvs.toml`. Opening the
socket with `Core\Net::bindDatagram` does not allow any sending by itself. A host that is not allowed
throws a `RuntimeError`. A message that is too large for one datagram throws an `IOError`, and
nothing is sent.
