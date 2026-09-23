`Core\Http\Response::tls(): ?Core\Http\TlsInfo` reports the session a reply arrived over, read through
members as every `Core` instance is (`rule:core-api/a-lifetime-is-an-object`): `version()`, `cipher()`,
`verified(): bool` — `true` only when the chain *and* the name were checked — `peerChain(): array<tainted
string>` as PEM, and the leaf's `subject()`, `issuer()` and `expiry()`. It answers `null` for a plain
`http` reply, because it had no session, and for one a test's table answered with no `tls` session
(`rule:testing/an-outbound-call-is-answered-from-a-table`). A table answer that names one reports the
session the test described with `Core\Test::tlsSession`.

`verified()` is the member this exists for. A deployment that relaxed verification for one partner host
(`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`) needs a way to assert, in a test and in
production telemetry, that every *other* call still verified — and without a reply-side answer the grant
is unobservable from inside the language.

The chain is `tainted` and the timings are not here: where a call's time went is the `http` trace event's
(`rule:observability/trace-events-carry-a-kind`), and a second surface for one measurement is the copy
that disagrees.
