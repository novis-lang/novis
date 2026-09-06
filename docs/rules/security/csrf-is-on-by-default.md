The server refuses an unsafe verb — the four that change state — without a valid token, using the
match it already made to know which handler is which. Generation and constant-time verification belong
to the protocol roster (`rule:security/protocol-roster`); this decides only the default, and the
default is on with nothing configured.

A route that legitimately needs no token — a webhook authenticated by signature, an API authenticated
by a bearer token — says so **in its own declaration**, per route, in the attribute that route already
carries: never a group and never a configuration key. Writing it on the route is what keeps the
exemption reviewable next to the handler it exempts.

Enforcing only when a session cookie is present was rejected within this rule. It sounds tighter,
since forgery can only target cookie-authenticated requests, but it makes protection depend on what
the client sent rather than on what the code says, which is harder to reason about and harder to test.
