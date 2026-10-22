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

**A body crosses a hop only inside one origin.** A hop follows RFC 9110 § 15.4: a `303` is followed
with a `GET` (a `HEAD` stays a `HEAD`), and a `301` or `302` after a `POST` is followed with a `GET`,
as every browser does. Neither sends a body, and neither sends the call's `Idempotency-Key`, since the
hop is not the request the key names. A `307` or `308` keeps the method and the body, so one that leaves
the origin while the call carries a body is not followed: the redirect is the answer. A form or a JSON
document carries the same credentials and personal data a header does.

No header value is written into a trace span, an access log record or an error message, on any hop. A
header guard that names the header and never the value is already how the transport reports a refusal,
and this is what makes that a tested property rather than a habit.

**Declined:**
- **A `307` or `308` to another origin followed with the body left out.** The method stays, so the
  far end gets a `PUT` or a `POST` the program never wrote, and it may act on it. Returning the
  redirect lets the program decide whether that host should have the body.
- **Following a cross-origin `307` or `308` with its body, as `fetch` does.** A browser sends no
  credential the page did not already hold for that origin; a server's call body is the program's
  own, and a `Location` header is the origin's to choose.
