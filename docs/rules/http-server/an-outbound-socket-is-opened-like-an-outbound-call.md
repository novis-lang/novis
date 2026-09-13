```
Core\Http\Client::openSocket(string|Core\Http\Target $url, ...$options): Core\Http\Socket
```

A WebSocket is opened through the door every outbound call passes, because the opening handshake **is** an
outbound call. The URL is `rule:security/outbound-url-is-a-sink`'s sink on both its spellings, so a
`tainted` value at that position is a compile-time diagnostic and `Core\Http::allowUrl` is the only way
past it; the `net.connect` grant, `rule:security/net-address-policy`, the pin over every approved address,
`connectTimeout`, the TLS policy options and their grants, a client identity, `headers` — a `secret` value
among them — and the operator's proxy tunnel
(`rule:http-server/an-outbound-proxy-is-operator-configured`) all apply exactly as they apply to `get`.
A door of its own would be a second implementation of the same policy, and the second copy is the one that
comes to be missing a check.

`ws` and `wss` join the scheme roster for this row alone
(`rule:http-server/allow-url-pins-the-address`): the request rows refuse them, and `openSocket` refuses
`http` and `https`, so what a URL is *for* is written in the row that takes it. `ws` is admitted as plain
`http` is, and there is no upgrade question, because a socket follows no redirect — a `3xx` answering an
upgrade is a `RuntimeError` naming the `Location` rather than a hop taken.

What the row answers is `Core\Http\Socket`, and what that answers is `Core\Socket\Message` — the shape a
server-side connection already answers in (`rule:concurrency/a-connection-is-a-root-isolate`), because one
RFC 6455 frame gets one shape in this language. `protocols` offers subprotocols as
`Sec-WebSocket-Protocol`, a `101` choosing one that was not offered is refused, and the socket reports the
one chosen. The bag carries the connection keys and not the exchange keys: there is no `body`, no
`followRedirects` and no retry trio, because a bag is a closed set and a key that could never do anything
is one a program would write and then wait for.

**A socket is never pooled.** It consumes its connection and nothing goes back to
`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`'s pool, because a connection
that has been upgraded can no longer carry a request.
