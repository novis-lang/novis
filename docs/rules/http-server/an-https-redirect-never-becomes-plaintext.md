A redirect hop from `https` to `http` is refused unless its target host is in a `net.downgrade` grant
*and* the call says `redirectToHttp: true`. A plain `http` URL asked for directly stays allowed, and the
refusal names the grant.

Without this, a server can strip a call's TLS with one `Location` header and the caller still sees a
`200` — the credentials are gone by then only because
`rule:http-server/a-cross-origin-redirect-drops-credentials` takes them, and the body still travels in
the clear. Following a redirect is already opt-in
(`rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`), so this is not a new decision for a
program to make: it is the one hop that opting in cannot silently include.

A direct `http` URL is untouched because the program chose it — an internal endpoint or a local service
is not being attacked by the code that asked for it — and because the address it reaches is judged the
same way either scheme is. Two conditions rather than one for `rule:security/capability-question-is-grant-and-scope`'s
reason: the deployment says where a downgrade may happen, and the call says it expects one here.
