A redirect hop to another origin — the scheme, the host or the port differing — drops `Authorization`,
`Cookie` and `Proxy-Authorization`, and with them every header whose value was `secret`. A hop inside one
origin keeps them all.

Forwarding a bearer token to whatever host a `Location` header names is a credential-exfiltration
primitive with one line of setup, and it is reached through a feature the caller asked for: following
redirects, which is why redirects are off until a count is written
(`rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`). The `secret` clause is what makes the
rule general rather than a list: a program's own API-key header is dropped for the same reason the three
named ones are, without anyone having to enumerate the spelling each partner invented
(`rule:security/secret-qualifier`).

No header value is written into a trace span, an access log record or an error message, on any hop. A
header guard that names the header and never the value is already how the transport reports a refusal,
and this is what makes that a tested property rather than a habit.
