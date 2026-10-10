`Strict-Transport-Security` is emitted when the effective scheme is `https`, and not otherwise. A
browser ignores it on a plaintext connection, so emitting it there is noise; and because the
server never terminates TLS itself, the effective scheme is the one a *trusted* proxy asserts
through `X-Forwarded-Proto`. With `trusted_proxies` empty the scheme is `http` and no HSTS is
sent — the safe direction for a header that cannot be revoked from a client. `localhost`,
`127.0.0.1` and `::1` are secure contexts in every current browser, so local development over
plain HTTP is unaffected.

The max-age is `[http.headers] hsts`, shipped as `365d`, and `false` disables it.
`hsts_subdomains` defaults to `false`.

**Declined:**
- **`hsts_subdomains = true` by default.** `includeSubDomains` is the HSTS setting that has
  actually taken deployments down: a sibling subdomain on plain HTTP becomes unreachable for the
  max-age, with no way to revoke it from the client, and whether every subdomain is TLS-only is
  knowledge the runtime does not have.
