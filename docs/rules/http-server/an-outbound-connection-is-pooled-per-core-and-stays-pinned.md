An outbound HTTP/1.1 connection is kept alive in a **per-core** pool, keyed by the pinned address, the
port, the scheme, the TLS server name, the call's client identity and the call's TLS policy. Per core
because a core owns its requests and a shared pool is a lock on the request path, and because a bound per
core is a bound this process can state: O(cores × `pool_idle`), never O(requests served), which is the
accounting `rule:security/db-pool-reset-is-a-boundary` already holds the database pool to.

**The key is the load-bearing part**, and every element of it is there because leaving it out lets reuse
hand a call a connection its own check would have refused. The pinned address keeps
`rule:http-server/allow-url-pins-the-address` true *through* reuse; the server name keeps two hosts behind
one address from sharing a session; the identity keeps a call presenting no certificate from reusing one
that did; the policy keeps a connection opened under a relaxed verification
(`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`) from ever serving a call that verifies. A
pool keyed on the URL's host — which is what most clients key on — would quietly undo the pin.

A call is approved for a **set** of addresses (`rule:http-server/an-outbound-call-tries-every-approved-address`),
and a held connection to any member of it serves the call: each member was approved, and each connection
is filed under the address it actually goes to. The key is still one address; what widens is the lookup,
which is one per address of the set rather than one on whichever address the walk would have tried first.

A connection returns to the pool **only** after its reply was read to the end under known framing —
`Content-Length` or chunked's last chunk — with no `Connection: close` from either side; on any doubt it
is closed, because a connection whose remaining bytes are unknown is one that will hand the next request
someone else's body. A reused connection that fails before the request's last byte is written is replaced
once **without spending an attempt**, a server closing an idle connection not being a failed request; any
later failure is an attempt under `rule:http-server/retry-is-opt-in-jittered-and-closed`. The two caps
are `[http.client] pool_idle`, the idle connections one core may hold, and `pool_idle_timeout`, how long
one may sit idle before it is closed — both `System` class, because they bound a core's memory rather
than a request's (`rule:config/three-changeability-classes`).
