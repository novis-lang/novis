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

**An upgrade is checked by its origin, on any verb.** A browser opens a WebSocket with a `GET` that
carries the visitor's cookies, so a request asking to upgrade that matched a route is refused when its
`Origin` names somewhere other than where it arrived, and passes with no `Origin`, as a write does. The
token half does not apply to it, key or no key, because a browser's handshake cannot carry the token
header; the write check a verb owes still follows, so an upgrade header takes nothing off it.

What a deployment that names no key therefore does **not** refuse is a covered request that sent no
`Origin` — every non-browser client, and so every API caller. Naming the key is what covers it. A key
written so the door cannot read it is refused at boot rather than left unarmed, because a deployment
whose configuration says the check is on and whose door verifies nothing is the one way this must not
fail.

A route that legitimately needs no token — a webhook authenticated by signature, an API authenticated
by a bearer token — says so **in its own declaration**, per route, in the attribute that route already
carries: never a group and never a configuration key. The same `csrf: false` turns off the upgrade
check, for a socket other sites are meant to open. Writing it on the route is what keeps the
exemption reviewable next to the handler it exempts.

**The refusal is the door's, and a program may only answer it.** In a program that calls
`Core\Router::dispatch`, the refused request still runs the entry, in refusal mode: `route()` answers
`null`, there is no body, and `dispatch` calls the gate's `refused` with `Refusal::Csrf` after setting
`403` (`rule:routing/a-gate-answers-around-every-handler`). The gate chooses the page, the status and a
log line, so a form sent after its session expired can show the user what happened. No handler runs,
and a gate that throws leaves the empty `403`. A program that does not call `dispatch` gets the empty
`403`, because it would read the refused request as an ordinary one with no route.

**Declined:**
- **Enforcing only when a session cookie is present.** It sounds tighter, since forgery can only
  target cookie-authenticated requests, but it makes protection depend on what the client sent rather
  than on what the code says, which is harder to reason about and harder to test.
- **Tokens available, and enforcement left to the program.** An unprotected state-changing route would
  look the same as a protected one at compile time and at run time.
- **A gate that lets a refused request through.** The exemption would then live in code a reviewer reads
  apart from the route. `csrf: false` on the route stays the only opt-out.
