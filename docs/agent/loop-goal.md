---
milestone: M8
---
# Loop goal 50 — an operator routes outbound calls through a forward proxy, and the grant says what the address policy can no longer see

A deployment whose egress goes through a corporate or cloud forward proxy can run Novis without giving up
the outbound policy. An operator writes `[http.client.proxy]`; every `Core\Http\Client` call then opens a
`CONNECT` tunnel through it. **By default the pin survives**: Novis resolves and checks the destination
exactly as it does today and asks the proxy to connect to the address it approved, with the host name
carried only in TLS's server name — so the proxy relays to where `rule:security/net-address-policy` said
yes. Where the proxy alone can resolve names, the operator says so in one written word, the policy
narrows to what can be judged from the URL's text, and the boot says that out loud every time. Nothing
reads `HTTPS_PROXY` from the environment.

## Why here

After goal `process-cache`, on the user's call: it was the one gap in the REST client's `Core` half that
the user placed in a goal of its own, because it deliberately weakens a guarantee and so gets a record of
its own. It needs goal `http-client`'s pool and hop — a tunnelled connection is pooled and re-pinned by
the same rules — and nothing from goal `process-cache`. Before goal `gap-zero` for that goal's standing
reason.

What it needs already built, all on disk once goal `http-client` is: the one place the transport connects
(`crates/nvs-stdlib/src/http/transport.rs:252`), the pin and its four questions (`crates/nvs-stdlib/src/http.rs`
`pin`), the TLS client that names the approved host and not the address (`transport.rs:36-54`), the
exception an operator-written endpoint already has (`docs/rules/security/net-address-policy.md:13-15`),
and the `_file` sibling a secret directive gets (`rule:config/a-secret-is-a-file-whose-content-is-the-value`).

## Stage 0 — the catch-up

- `crates/nvs-stdlib/src/http/transport.rs` — its module doc says a connection is made to the pinned
  address directly. Stage 3 says what a tunnel changes and what it does not.
- `crates/nvs-stdlib/src/http.rs` — the pin's doc names no proxy. Stage 4, where `resolve = "proxy"` makes
  the pin a check on the text alone.
- [0058](../decisions/0058.md) § *Consequences* records a proxy as "a cost this ADR accepts" and that
  "the grant syntax should make that explicit". Frozen: this goal's record is the answer it asked for, and
  0058 is not edited.

## Stage 1 — the floor

Goal `process-cache`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the record

One new record, and no other number. Its body is § *Standing decisions* below, argued, and it answers
0058's open sentence. It creates two rules, both `designed`:

| Rule | Says |
|---|---|
| `http-server/an-outbound-proxy-is-operator-configured` | `[http.client.proxy]` is the only source, `System`-class, never the environment; its keys and their refusals |
| `http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise` | `resolve = "local"` keeps the pin; `resolve = "proxy"` narrows the policy to the URL's text, and says so at every boot |

It modifies `security/net-address-policy` by one sentence: a configured proxy is an operator-written
endpoint and is not asked the table's question, and with `resolve = "proxy"` the destination's resolved
address is not asked it either, because Novis never sees it.

## Stage 3 — the keystone: a tunnel that keeps the pin

`crates/nvs-stdlib/src/http/transport.rs`. With `[http.client.proxy] url` set and `resolve = "local"`,
the transport connects to the proxy — its address pre-approved by the operator's writing — sends
`CONNECT <approved address>:<port> HTTP/1.1` with `Host` naming the same, waits for a `2xx` under the call's
`connectTimeout` and `deadline`, and then speaks exactly what it speaks today over the tunnel: TLS with
the server name the launderer approved (`transport.rs:45-54`), or plain HTTP for an `http` URL. A plain
`http` URL is tunnelled too, so there is one mechanism and never an absolute-form request line.

- **The pool key gains the proxy**, so a direct connection and a tunnelled one are never confused, and a
  reload that changes the proxy drops the connections made through the old one.
- **A `407` is a `RuntimeError` naming proxy authentication; any other refusal of `CONNECT` is an
  `IOError`**, and neither is retried by `retryAttempts`, which retries answers from the destination.
- A redirect hop is re-pinned exactly as today and tunnelled the same way.

## Stage 4 — the proxy resolves, a bypass list, and authentication

- **`resolve = "proxy"`.** `CONNECT <host>:<port>`; Novis judges the URL's text — scheme, grant — and
  cannot check the address the proxy resolves. The boot writes one `Warn` record naming the block and the
  rule every time it starts with this set, and `nvs config dump` renders it beside that rule's id. The
  policy is then the proxy's to enforce, which is 0058's sentence made visible rather than implied.
- **`bypass`**: an array of host names, each exact or with a leading `.` for a suffix, reached directly
  and under the full policy. No wildcard, no CIDR and no port.
- **`username` and `password`**, the password with its `password_file` sibling in the secret registry
  (`rule:config/a-secret-is-a-file-whose-content-is-the-value`), sent as `Proxy-Authorization: Basic` on
  `CONNECT` only — never to the destination, never in a trace, a log record or an error message.

## Stage 5 — the rulebook

Flip stage 2's two rules to `shipped`, with `guardedBy` filled from this goal's tests, and `python
tools/rules.py --render`.

## Standing decisions

- **Settled with the user**: a forward proxy is in, as its own goal, configured by the operator and never
  by a program or the environment.
- **`resolve` is mandatory, with no default**, for `rule:config/scope-has-no-default`'s reason: both
  answers are commonly correct, and either default silently does the wrong thing in somebody's
  production — `local` fails outright in a network where only the proxy resolves, and `proxy` quietly
  gives up the pin everywhere else. A block without it refuses the boot naming both words.
- **Only `Core\Http\Client` is proxied.** `Core\Net`, a database, the shared cache tier and mail connect
  directly: each is either a raw socket the program asked for by address, or an endpoint the operator
  already wrote.
- **`http://` proxies only**: `CONNECT` over plain TCP to the proxy. A TLS connection *to* the proxy is not
  this goal.
- **Config**: `[http.client.proxy]` — `url`, `resolve`, `bypass`, `username`, `password` /
  `password_file` — is `System`, because where every outbound byte goes is not a request's decision, and
  `Reload`, because the next call reads it and the pool drops what the old value made.
- **What it spends**: one `CONNECT` round trip per new connection, then nothing per request, since the
  tunnel is pooled like any connection; per core, the pool's existing cap. Nothing is held per request
  beyond today's.
- **ADR slots**: the one record of stage 2.
- **Not this goal**: a TLS connection to the proxy, SOCKS, PAC files, NTLM or Kerberos proxy
  authentication, per-app proxies, and reading `HTTP_PROXY`, `HTTPS_PROXY` or `NO_PROXY` — the environment
  is read-only ambient state `rule:security/no-cross-request-state` keeps out of behaviour. A session that
  finds one on its path writes it to the handoff's `## Backlog`.
