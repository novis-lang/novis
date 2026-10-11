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

`tlsSession` is what puts the seven `Core\Http\TlsInfo` members within a Novis program's reach without
a network, so each has its example, bench and attack (`rule:testing/feature-proofs`), and a test of a
program that audits its outbound sessions can be written. Every option has a default: `TLSv1.3` over
`TLS_AES_128_GCM_SHA256` (`TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256` for `TLSv1.2`, the AES-128-GCM
suite each version uses with the leaf's ECDSA key), verified, subject `CN=example.com`, issuer
`CN=Novis Test CA`, and an expiry 90 days after `Core\Time::now()`, so a test with a fixed clock gets
the same value every run. A cipher must be one this build negotiates over that version; a subject or
issuer takes the keys `CN`, `O`, `OU`, `C`, `ST` and `L`, each at most once; an expiry lies between
1950 and the end of 9999. Anything else is a `LogicError` naming the field. The chain is two
certificates, leaf first, each over a fresh P-256 key, because a real server sends its leaf and an
intermediate and a `peerChain()` example should show that order. A prefix row whose URL names no
scheme is not checked for `http://`.

Neither member refuses a call outside a test, for the reason `answerHttp` already gives: nothing
trusts what they build, a reply that reports the session came from the program's own table, which has
already taken it off the network, and no request input reaches the certificate issuer.

- **Memory:** one copy of a described chain, about 1 KB, per registered answer that names a session,
  for the life of the test's context, and a fresh `Core\Http\TlsInfo` per reply that answer serves,
  released with the reply. A program outside a test pays nothing.

**Declined:**
- **Skipping the Novis proofs for `Core\Http\TlsInfo`.** Rust tests alone leave seven public members
  with no example a reader can learn from, and the audit test above still could not be written.
- **A local TLS test server beside `Core\Test::serverUrl`.** A real handshake would need a certificate
  and trust setup and far more code than reading a session back needs. It stays open as a question of
  its own.
- **The session's fields as a nested bag in `answerHttp`'s `tls` option.** An option is never itself a
  bag, so that form would need a compiler change; a member of its own takes an ordinary bag.
- **Seven flat `tls*` options on `answerHttp` in place of the one `tls` option.** They would give
  that member ten options instead of four.
- **A PEM the test pastes in.** It puts a block of base64 in every example, and one certificate
  compiled into the binary could not show three different uses of `subject`.
