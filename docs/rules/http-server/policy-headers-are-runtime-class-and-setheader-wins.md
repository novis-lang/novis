Every directive under `[http.headers]`, `[http.cors]` and `[http.cookies]` is
`rule:config/three-changeability-classes`' **`Runtime`** class: `nvs.toml` states the default a
request starts with, a request may set any value for itself through `Core\Config::set`, and the
change is discarded when the request ends (`rule:config/a-runtime-set-is-request-local`). The next
request on the same core sees the configured value. `Core\Response::setHeader` additionally
overrides a policy-owned header on **one** response with no configuration involved at all: a
header the response already carries is left alone when the policy set is filled in.

This costs nothing in security because the alternative buys nothing. A request can already write
any response header it likes through `setHeader` — that is what a response object is. A
compromised handler can therefore turn its own response's protections off, and could before any
of these blocks existed. Per-request override is what keeps the policy usable: an API route with
its own CORS policy and an embeddable route that permits framing are both ordinary, and neither
needs a second configuration mechanism.

A policy-owned header replaced through `setHeader` is **not** logged.

**The framing headers are not policy, and `setHeader` throws `LogicError` for them.** `Content-Type`
is the body member's (`rule:security/response-body-is-one-typed-member`). `Content-Length`,
`Transfer-Encoding`, `Connection` and `Upgrade` are the server's: it writes each from the body it
sends and the connection it holds. `hyper` frames a body by a `Content-Length` it is handed, so a
program's that disagrees with the body desynchronises the connection, a response split with no
newline in it. The server drops a declared framing header too, so a declaration that reaches it
some other way changes nothing.

**Declined:**
- **`RuntimeTighten` policy blocks.** Narrowing-only would forbid the legitimate case (one API
  route with an open CORS policy beside an application that has none) while stopping nothing.
  `RuntimeTighten` is for grants, where "may drop rights, never add them" is the whole mechanism
  (`rule:config/three-changeability-classes`); a response header is not a grant.
- **Logging an override.** The header is ordinary output the request wrote deliberately, and a
  line per response on any route that customises one is noise that trains people to ignore the log.
- **Admitting a `Content-Length` that matches the body.** The server already writes that value,
  so the only `Content-Length` a program could add is a wrong one.
