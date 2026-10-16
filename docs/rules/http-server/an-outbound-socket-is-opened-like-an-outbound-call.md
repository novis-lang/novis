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

**The row is named `openSocket`** because `rule:core-api/verb-lexicon` narrows it to one name. `connect`
is unavailable: every other row on the class is named for an HTTP method, `CONNECT` is one, and
`connectTimeout` and `connectTo` already spend the word on reaching a host. Bare `open` is this
language's verb for a resource a program then holds and closes, but alone on this class it does not say
what is opened, and this is the only row whose result outlives the call.

`ws` and `wss` join the scheme roster for this row alone
(`rule:http-server/allow-url-pins-the-address`): the request rows refuse them, and `openSocket` refuses
`http` and `https`, so what a URL is *for* is written in the row that takes it. `ws` is admitted as plain
`http` is, and there is no upgrade question, because a socket follows no redirect — a `3xx` answering an
upgrade is a `RuntimeError` naming the `Location` rather than a hop taken.

What the row answers is `Core\Http\Socket`, and what that answers is `Core\Socket\Message` — the shape a
server-side connection already answers in (`rule:concurrency/a-connection-is-a-root-isolate`), because one
RFC 6455 frame gets one shape in this language. `protocols` offers subprotocols as
`Sec-WebSocket-Protocol`, a `101` choosing one that was not offered is refused, and the socket reports the
one chosen; any other answer than a `101` is a `RuntimeError` naming the status. The bag carries the connection keys and not the exchange keys: there is no `body`, no
`followRedirects` and no retry trio, because a bag is a closed set and a key that could never do anything
is one a program would write and then wait for.

**A socket is never pooled.** It consumes its connection and nothing goes back to
`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`'s pool, because a connection
that has been upgraded can no longer carry a request.

**The conversation is `receive`, `send`, `sendBytes`, `close` and `protocol`.** `send(string)` and
`sendBytes(bytes)` are two classified parameters, as on `Core\Socket`, because a `string|bytes` union
carries no classification, so it would refuse a `tainted` argument and make `send` a sink by accident.
`receive` answers `?Core\Socket\Message` with `text` or `bytes` filled and `tainted`, and `null` once the
peer has closed. A sent frame is not a sink (`rule:security/sink-predicate`): it is no instruction on
this side of the wire, so a program may forward what it received without laundering it. RFC 6455's
masking, fragmentation, UTF-8 validation and close handshake are `tungstenite`'s client half, run over
the stream the transport already holds.

**Declined:**
- **A door of its own, `Core\Socket::connect`.** It groups the socket surface in one namespace, and asks
  every question the pin, the grant, the TLS options and the proxy answer a second time.
- **A second message class for the outbound direction.** It would differ from `Core\Socket\Message` only
  in where the bytes came from, and the two would disagree the first time either was touched.
- **WebSocket over HTTP/2 (RFC 8441).** The outbound client speaks HTTP/1.1, and this changes with it.
- **STOMP, Socket.IO and GraphQL over WebSocket in the runtime.** Each is frames with a convention on
  top, which is what a package is for (`rule:core-api/tier-placement`).
