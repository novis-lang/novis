The server refuses an unsafe verb — the four that change state — without a valid token, using the
match it already made to know which handler is which. Generation and constant-time verification belong
to the protocol roster (`rule:security/protocol-roster`); this decides only the default, and the
default is on with nothing configured.

**The refusal has two grounds, and what arms each of them differs.** A covered request whose `Origin`
names somewhere other than where it arrived is refused with nothing configured at all — that is the
forgery this is named for, and it reads only headers the request brought. A covered request without a
token this deployment issued, bound to the session it rides under, is refused as soon as `[http]
csrf_key` names the key that says what a valid token is; until it does, there is no expected token for
any request to be missing. A valid token passes whatever the origin says, because it proves the page
the request came from was this application's.

What a deployment that names no key therefore does **not** refuse is a covered request that sent no
`Origin` — every non-browser client, and so every API caller. Naming the key is what covers it. A key
written so the door cannot read it is refused at boot rather than left unarmed, because a deployment
whose configuration says the check is on and whose door verifies nothing is the one way this must not
fail.

A route that legitimately needs no token — a webhook authenticated by signature, an API authenticated
by a bearer token — says so **in its own declaration**, per route, in the attribute that route already
carries: never a group and never a configuration key. Writing it on the route is what keeps the
exemption reviewable next to the handler it exempts.

Enforcing only when a session cookie is present was rejected within this rule. It sounds tighter,
since forgery can only target cookie-authenticated requests, but it makes protection depend on what
the client sent rather than on what the code says, which is harder to reason about and harder to test.
