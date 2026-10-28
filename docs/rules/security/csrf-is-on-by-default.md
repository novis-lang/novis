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

**The origin half covers every unsafe verb, matched or not.** A program with no `#[Route]` table,
and a path a routed program's table does not claim, get the same origin refusal a matched row gets.
The origin is judged before `Core\Router::dispatch` asks whether a route matched, so a cross-origin
unsafe write to an unclaimed path is `Refusal::Csrf`, never `NotFound`, and `NotFound` is a
same-origin request, one with no `Origin`, or a safe verb.
The token half applies only to a matched row, because the opt-out is written on a route: a program
with no table could not exempt a webhook from it, and every write it serves under a key would be
refused.

**The session a token is bound to is read the way `Core\Session::start` reads it**: the cookie
`[session] cookie` names, found on any `Cookie` line, pair by pair, with one reader for both. Two
readers that split the header differently would check a token against one session while the program
opens another. **A request with no session passes no token**, from any origin. A token for no session
is what a page shows a visitor who has none, so anyone can load one, and accepting it would let
another site sign a visitor in to an account the attacker chose.

**A page issues a token with `Core\Csrf::token`**, after `Core\Session::start`. It binds the token to
the identifier the response's session cookie carries, under `[http] csrf_key` read by the door's own
reader, so the token it renders is one the door accepts on the next request. For a visitor with no
session, the call is the session's first write: it draws the identifier and sends the cookie, so a
login form shown to a new visitor is bound to an identifier too. `token` before `start` throws naming
`start`, and with no key it throws naming `[http] csrf_key`. `Core\Csrf::issue` stays for a program
that binds a token to a value of its own and checks it itself with `verify`.

**The token rides in the `x-csrf-token` header, a `_csrf` query parameter or an urlencoded form's
`_csrf` field**, and the first of the three present decides. A plain HTML form therefore needs no
script. The form body is read only for a checked `application/x-www-form-urlencoded` request that
sent neither of the other two, up to `[limits] request_body`, by the request's own task before the
handler runs, and the handler reads the same form afterwards. A body past the limit is refused as a
missing token. A multipart body is never read for a token: a file upload would have to be buffered
whole before the check, on the sender's terms, so a multipart write carries its token in the header or
the query. **What it spends:** up to `[limits] request_body` per such request in flight, attributed to
that request, and bytes the handler's own form parse would have buffered anyway.

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
- **Reading a multipart body for its `_csrf` field.** Every upload would be buffered before the check,
  up to `[limits] upload_total`, before anything decides whether to run it.
- **A gate that lets a refused request through.** The exemption would then live in code a reviewer reads
  apart from the route. `csrf: false` on the route stays the only opt-out.
