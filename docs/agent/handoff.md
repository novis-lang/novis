# Handoff

## State

**Goal `outbound-proxy` — an operator routes outbound calls through a forward proxy, and the grant
says what the address policy can no longer see. Stage 3's config half is on disk; nothing dials a
proxy yet.**

[0182](../decisions/0182.md) and its two rules —
`rule:http-server/an-outbound-proxy-is-operator-configured` and
`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise` — are accepted and
`designed`; stage 5 flips both to `shipped` and fills `guardedBy`. Both are now in the goal's base
`[context] rules`, in `loop-goal.toml` and in `docs/agent/goals/50-outbound-proxy.toml` alike.

`[http.client.proxy]` is a block an operator can write: `HttpClientProxy` in the typed tree, one
`System`/`Reload` row in the registry, a commented-out section in `default.toml`, and a
`secret::SECRETS` row — the roster's first block that is not a map, so its one site is the unnamed
one. `nvs_config::http::validate` refuses `E0643` (no `resolve`, naming both words), `E0644` (a third
word), `E0645` (a `url` this client cannot dial, an absent one included) and `E0646` (a `bypass`
entry that is not a host name); `advise` writes `W1010` at every boot under `resolve = "proxy"`.
Three of stage 4's named `-p nvs-config` cases are green with it.

`HttpClientProxy::username` carries an `[unread:]` trailer, which `tools/directives.py --check`
requires until the transport reads it; the next slice deletes both it and `default.toml`'s
`# NOT IMPLEMENTED` note above the key.

## Next group

**Stage 3: the tunnel itself** — one file set: `crates/nvs-stdlib/src/http/transport.rs`,
`crates/nvs-stdlib/src/http/pool.rs`.

- [ ] **The `CONNECT` tunnel under `resolve = "local"`** — in `one`, between the TCP connect at
      `crates/nvs-stdlib/src/http/transport.rs:1403` and the TLS wrap at
      `crates/nvs-stdlib/src/http/transport.rs:1426`: dial the proxy instead of the approved address,
      write `CONNECT <addr>:<port>` for the address the pin approved, read the head, and hand the
      same stream to `NvsTls::over_policy` with the host name unchanged. `bypass` is matched on the
      URL's host text before any of it. How the transport reaches the snapshot is the first question
      — `Call` is at `crates/nvs-stdlib/src/http/transport.rs:142`, and
      `nvs_config::http::RESOLVE_LOCALLY`/`RESOLVE_AT_THE_PROXY` are the two words.
      `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`.
- [ ] **The pool key gains the proxy** — `crates/nvs-stdlib/src/http/transport.rs:1503`, whose doc
      already says why every question the door asked is in the key: a tunnelled connection and a
      direct one to the same origin are two keys.
      `rule:http-server/an-outbound-proxy-is-operator-configured`.
- [ ] **A refused `CONNECT` is two faults, and neither is retried** — a `407` is a `RuntimeError`
      naming proxy authentication and anything else is an `IOError`; `retryable` at
      `crates/nvs-stdlib/src/http/transport.rs:1932` is what must not treat either as an answer.
      `rule:http-server/an-outbound-proxy-is-operator-configured`.

## Backlog

- `nvs config dump` renders `resolve` beside `rule:security/net-address-policy`'s id — 0182 § 4's
  second bullet, `crates/nvs-config/src/export.rs`, stage 4.
- 0182 § *Diagnostics* calls the boot announcement an observability record rather than a diagnostic.
  It landed as `W1010` on `W1009`'s footing, because the check that names it is `-p nvs-config` and
  that crate emits warnings, not records. Worth one look at stage 5.
- `no_proxy_environment_variable_is_ever_read` is stage 4's, in `-p nvs-stdlib`: nothing in
  `nvs-config` reads the environment, so the case belongs where a transport could have.
