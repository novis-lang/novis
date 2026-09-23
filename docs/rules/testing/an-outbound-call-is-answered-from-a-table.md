Once a test registers one answer with `Core\Test::answerHttp(string $url, uint $status, {json?, body?,
headers?, tls?})`, **every** outbound call that test makes is answered from the table, and an unmatched
one throws a `LogicError` naming the URL. A `$url` matches exactly, or as a prefix ending in `*`, and
`Core\Test::sentHttp(): array<Core\Test\SentRequest>` hands back what the program sent, in order.

An answer reports no TLS session unless its `tls` option names one:
`Core\Test::tlsSession({version?, cipher?, verified?, subject?, issuer?, expiry?}): Core\Http\TlsInfo`
describes a session over a real leaf certificate for `subject`, signed by a CA certificate for
`issuer`, and every reply that answer serves reports it. A `tls` session on an `http://` URL is a
`LogicError`, because a reply over plain `http` has none.

All-or-nothing is the whole point. A table that answers the calls it knows and lets the rest reach the
network is a suite that passes on a laptop, talks to a partner's production API from CI, and says nothing
about either — so the first registered answer takes the test off the network entirely. A faked call is
still judged where the judging is about *text* — the scheme and the outbound grant, per
`rule:security/outbound-url-is-a-sink` — while resolution and the address check are skipped, there being
no connection for an address to be pinned to
(`rule:http-server/allow-url-pins-the-address`). It files no `http` trace event either: nothing crossed a
network.

`SentRequest` is readonly — `method()`, `url()`, `header()` and `body()` — and nothing on it is
`tainted`, unlike every member of a real reply: a sent request is text the program itself authored, which
is the one question `rule:security/tainted-qualifier` answers. This is what makes the client's own
conformance cases writable with no listener and no outbound grant, the same way
`rule:testing/in-process-request` makes a request testable with no socket; what it cannot reach is
everything below the table — the pool, the framing, a content coding, a redirect hop, a handshake — which
is proved against a loopback origin instead. A described session is what a handshake *reports*, not a
handshake: nothing verifies its chain and no anchor trusts its CA.
