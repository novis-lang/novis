Redirects are **not followed by default**: `[http.client] max_redirects` ships as `0`, and a
`followRedirects` count at the call is the only way to raise it. When a hop is taken it is
re-checked and re-pinned by the same procedure the first URL passed
(`rule:http-server/allow-url-pins-the-address`), and a redirect to a denied address **fails the
request** rather than being silently dropped from the chain — a redirect is the standard way to
defeat a check applied only to the first URL. A redirect that arrives when no hop was asked for
is simply the answer.

A **retry** is the opposite case and must not re-resolve: every attempt of a retried call reuses
the `Target` the launderer pinned, so retrying opens no second resolution for a rebinding attack to
poison, and a retried call performs exactly one DNS resolution. A redirect hop re-resolves and
re-checks; a retry attempt does neither.

Both the redirect chain and every retry attempt are covered by one `deadline`
(`rule:http-server/one-deadline-covers-the-whole-call`).
