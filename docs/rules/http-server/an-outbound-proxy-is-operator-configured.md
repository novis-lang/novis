A `Core\Http\Client` call leaves through a forward proxy when, and only when, an operator has written
`[http.client.proxy]` in `nvs.toml`: there is no call option, no client option, no program-side spelling,
and **no environment variable is read** — not `HTTP_PROXY`, not `HTTPS_PROXY`, not `NO_PROXY`, in any case
spelling.

The block is `System` class and `Reload` (`rule:config/three-changeability-classes`,
`rule:config/reloadability-is-its-own-field`): `Core\Config::set` fails on every key, and a changed value
takes effect on the next call, with the pool's key carrying the proxy so nothing the old value made can
serve one.

| Directive | Ships | Refuses |
|---|---|---|
| `url` | — | anything but an `http://` URL with a host |
| `resolve` | **none — mandatory** | a missing value, and any word but `local` or `proxy` |
| `bypass` | `[]` | an entry with a port, a scheme, a `*` or a `/` |
| `username` | — | — |
| `password` / `password_file` | — | both of the pair set |

Where every outbound byte goes is a deployment's decision, not a request's. A per-call proxy would be a
per-call way to choose who resolves the destination, and therefore a per-call way to narrow
`rule:security/net-address-policy` — the widening that rule exists to refuse. The environment is the same
argument with a worse blast radius: read-only ambient state, shared by every request in the process,
settable by anything that can set a variable for it, and recorded nowhere in the deployment's own
configuration (`rule:security/no-cross-request-state`).

**Only `Core\Http\Client` is proxied.** `Core\Net`, a database, the shared cache tier and mail connect
directly — each is either a raw socket the program asked for by address, which has no notion of a tunnel,
or an endpoint an operator already wrote into root-owned configuration.

**The proxy URL's scheme is `http`**: `CONNECT` over plain TCP, with `https://` refused at boot. TLS *to*
the proxy is a second trust decision and has no spelling here. Every destination is tunnelled, `http` ones
included, so there is one mechanism and no path on which the proxy is handed a full request in
absolute form.

`bypass` matches the URL's host text — each entry exact, or with a leading `.` for a suffix — and a
bypassed destination is reached directly and under the full address policy. No CIDR, no wildcard and no
port: matching happens before any resolution, and a range in that position hands back more than the
operator can see they are handing back.

`username` and `password` become `Proxy-Authorization: Basic` on the `CONNECT` request **alone** — never
sent to the destination, never re-sent on a redirect hop, and never written into a trace event, a log
record or an error message. `password` is a secret directive with the usual `_file` sibling
(`rule:config/a-secret-is-a-file-whose-content-is-the-value`). A `407` is a `RuntimeError` naming proxy
authentication and any other refusal of `CONNECT` is an `IOError`; neither is retried by `retryAttempts`
(`rule:http-server/retry-is-opt-in-jittered-and-closed`), which retries an answer from the destination,
and a proxy that refused the tunnel is not one.
