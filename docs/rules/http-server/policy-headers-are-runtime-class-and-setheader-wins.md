Every directive under `[http.headers]`, `[http.cors]` and `[http.cookies]` is
`rule:config/three-changeability-classes`' **`Runtime`** class: `nvs.toml` states the default a
request starts with, a request may set any value for itself through `Core\Config::set`, and the
change is discarded when the request ends (`rule:config/a-runtime-set-is-request-local`). The next
request on the same core sees the configured value. `Core\Response::setHeader` additionally
overrides a policy-owned header on **one** response with no configuration involved at all: a
header the response already carries is left alone when the policy set is filled in.

This costs nothing in security because the alternative buys nothing. A request can already write
any response header it likes through `setHeader` — that is what a response object is — so making
the policy narrowing-only would forbid the legitimate case (one API route with an open CORS policy
beside an application that has none; one embeddable widget route that permits framing) while
stopping nothing. `RuntimeTighten` is for grants, where "may drop rights, never add them" is the
whole mechanism; a response header is not a grant.

A policy-owned header replaced through `setHeader` is **not** logged. It is ordinary output the
request wrote deliberately, and a line per response on any route that customises one is noise
that trains people to ignore the log.
