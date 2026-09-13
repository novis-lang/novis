A redirect hop to another origin — the scheme, the host or the port differing — carries none of the
caller's own headers: `Authorization`, `Cookie` and `Proxy-Authorization` go, and with them every header
whose value was `secret` and every other one the program wrote. A hop inside one origin keeps them all.
This client's own headers are composed per hop either way, so the offer, the framing of a body and
`traceparent` cross a hop the caller's headers do not.

Forwarding a bearer token to whatever host a `Location` header names is a credential-exfiltration
primitive with one line of setup, and it is reached through a feature the caller asked for: following
redirects, which is why redirects are off until a count is written
(`rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`). The `secret` clause is what makes the
rule general rather than a list: a program's own API-key header is dropped for the same reason the three
named ones are, without anyone having to enumerate the spelling each partner invented
(`rule:security/secret-qualifier`).

**Every header and not a list, because `secret` is erased before codegen.** The qualifier is checked
once and costs nothing at run time (`rule:security/secret-qualifier`), so which header value carried one
is not a question the transport can ask: a `secret string` and a plain one are the same bytes by the time
a request is composed. Dropping all of them is the one way to drop every `secret` one without spending a
representation the language does not spend. What that costs is a program's own `Accept` on a hop it asked
to follow; what it buys is that no credential under a name nobody enumerated crosses to a host a
`Location` chose, and the priority ordering is what decides between those two.

No header value is written into a trace span, an access log record or an error message, on any hop. A
header guard that names the header and never the value is already how the transport reports a refusal,
and this is what makes that a tested property rather than a habit.
