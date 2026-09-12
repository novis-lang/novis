# Handoff

## State

**Goal `outbound-proxy` — an operator routes outbound calls through a forward proxy, and the grant says what the address policy can no longer see — has just started; nothing of it has landed yet.** Goal `process-cache`'s whole list is this goal's Stage 1 floor.

**The design is settled with the user and is written into the goal's § *Standing decisions*; the record
that states it is not written yet.** Stage 2 writes it. The things not to re-decide: the operator
configures the proxy and nothing else does; `resolve` is a mandatory word with no default; `resolve =
"local"` tunnels to the address Novis approved, so the pin survives; `resolve = "proxy"` narrows the
policy to the URL's text and warns at every boot; only `Core\Http\Client` is proxied.

## Next group

**Stage 2: the record** — one file set: `docs/decisions/`, `docs/rules/http-server*`,
`docs/rules/security*`.

- [ ] **The record** — the next free number in `docs/decisions/`, `changes.creates` the two rules the
      goal's stage 2 table names, `changes.modifies` `security/net-address-policy`. It answers 0058's
      sentence that the grant should make a proxy explicit.
- [ ] **The two rule fragments and their JSON entries**, both `designed`, and the one-sentence amendment
      to `security/net-address-policy`, then `python tools/rules.py --render`.

## Backlog

- Stage 3 — the tunnel. `crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-config/src/directive.rs`.
  The keystone. Its own session.
- Stage 4 — `resolve = "proxy"`, `bypass`, and authentication. `transport.rs`, `http.rs`, and the secret
  registry in `nvs-config`. Shares `transport.rs` with stage 3.
- Stage 5 — the flips. `docs/rules/` only.
- When this goal's last check goes green the driver takes goal `websocket-client`.
