`X-Forwarded-Proto` from a trusted peer sets the effective scheme, and feeds exactly two things: `Core\Request::scheme()` and HSTS emission. HSTS is emitted when the effective scheme is `https`, not when a TLS connection was terminated — Novis never terminates one — and never on the assertion of an untrusted peer.

**A trusted peer asserts the scheme only when it also forwarded an `X-Forwarded-For` chain.** A peer that wrote no chain did not act as a proxy for this request: it is a client on a trusted address, such as another program on a trusted loopback, and its `X-Forwarded-Proto` is not read. The proxy must overwrite `X-Forwarded-Proto` on every request, never pass the client's through; the reverse-proxy page sets it in both nginx blocks. The cost is that a proxy that writes `X-Forwarded-Proto` but no `X-Forwarded-For` is served as `http`, with no HSTS, which is the safe direction.

**Declined:**
- **Detecting a proxy that appends `X-Forwarded-For` and passes the client's `X-Forwarded-Proto` through.** Nothing on the wire tells that header from one the proxy wrote. Overwriting it is the proxy's configuration.
- **Reading the scheme from the leftmost or the untrusted hop's position in the chain.** `X-Forwarded-Proto` carries no hop to align with `X-Forwarded-For`, so the nearest proxy's last token is the one value whose author is known.

It does **not** feed redirects: `Core\Response::redirect` emits `Location` as given and relative is permitted, so no scheme is ever reconstructed and the downgrade-and-loop bug behind a TLS-terminating proxy is removed rather than handled. It does not feed the cookie `Secure` flag, which the response policy sets unconditionally.

There is **no `X-Forwarded-Host` and no absolute-URL generation from `Host`**. `Core\Router::url` answers with a path (`rule:routing/matching-is-not-dispatching`), so Novis never needs to know its own external origin, and deriving one from a header is host-header injection. An external origin, where an application needs one, is configuration: `rule:routing/an-origin-is-per-mount-and-checked-at-boot`.
