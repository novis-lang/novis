Sends one message over UDP to a host and a port. The message is sent whole, as one datagram, and
the method returns how many bytes it sent.

`send` does not wait for an answer and does not check that the message arrived. UDP gives no such
promise. If the other side must answer, it sends its own message back, and you read it with
`receive`.

Your program may send to any public host with no `nvs.toml`. A `connect` list in the
`[capabilities.net]` block allows only the hosts it names. A host that is not allowed throws a
`RuntimeError`. A message that is too large for one datagram throws an `IOError`, and
nothing is sent.
