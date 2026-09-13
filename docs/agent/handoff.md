# Handoff

## State

**Goal `http-client` — a program talks to a real API. Stages 1–9 are on disk; stage 10's
configuration half and its option half are landed, and the verifiers behind them are not.**
[ADR 0180](../decisions/0180.md) § 11 is the record this stage executes.

The five keys are rows of the shared bag — `tlsCa`, `tlsPin`, `tlsVerifyHost` and `tlsVerify` at ABI
slots 14–17 and `tlsMinVersion` at 18 (`crates/nvs-stdlib/src/http.rs:1049`) — each with a reference
card, and `judge_trust` (`crates/nvs-stdlib/src/http.rs:1923`) asks each relaxing option's
`capabilities.tls` grant of the request's own snapshot beside the other judges, before any socket is
opened. It throws a `LogicError`, which is ADR 0180 § *Diagnostics*' class for this one and not the
door's usual `RuntimeError`: `nvs_runtime::capability::require_as` is that door with the class as a
parameter, so the sentence an operator reads is still `denial`'s. `tlsMinVersion` needs no grant and
may only raise `[http.client.tls] min_version`.

Nothing reads those options after the judging. `nvs_host::tls` has no relaxing verifier and no
policy value, and neither `Call` nor `pool_key` carries one, so stage 10's `nvs-host` check is an
open item rather than a regression.

Nothing is blocked.

## Next group

**Stage 10: the verifiers, and the policy through the pool** — one file set:
`crates/nvs-host/src/tls.rs`, `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **The policy value and the verifiers, inside the one client** — a relaxing policy beside
      `ClientPolicy` (`crates/nvs-host/src/tls.rs:472`) and the configuration built from it beside
      `anchors_from` (`crates/nvs-host/src/tls.rs:695`), plugged into `rustls`'s custom-verifier seam
      where `verifying` stops today (`crates/nvs-host/src/tls.rs:735`). A caller hands in the value
      and never a session (`rule:security/one-tls-client`), and every relaxed verifier still checks
      the handshake signature. The five tests the stage's check names go beside `issued()` and the
      loopback TLS peer (`crates/nvs-host/src/tls.rs:901`); `x509-parser`, `spki` and `der` are
      already in `Cargo.lock`, so a `sha256//` SPKI pin needs no new dependency.
- [ ] **The policy reaches the handshake and the pool key** — built where the options are already
      read (`crates/nvs-stdlib/src/http.rs:2573`), carried on `Call`
      (`crates/nvs-stdlib/src/http/transport.rs:131`) and written into `pool_key`
      (`crates/nvs-stdlib/src/http/transport.rs:1349`), which is what
      `a_relaxed_connection_never_serves_a_strict_call_from_the_pool` asserts.
      `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`.
- [ ] **The boot prints every relaxed grant, one line per grant and host** — ADR 0180 § 11's last
      bullet, and the last thing stage 10 owes. `[http.client.tls]`'s own boot advisory
      (`crates/nvs-config/src/http.rs:490`) is the precedent and the shape;
      `Cap::takes_true_spelling`'s six are the grants to walk
      (`crates/nvs-config/src/capability.rs:99`).

## Backlog

- The class split needs one sentence somewhere: `rule:security/denial-is-a-runtime-error` says a
  denied capability throws `RuntimeError`, and ADR 0180 § *Diagnostics* files a relaxing option
  whose host no grant lists as a `LogicError`, which is what the code does —
  `docs/rules/security/denial-is-a-runtime-error.md`.
- `tlsCa` beside a plain `http` URL is granted-and-ignored rather than refused; whether that owes a
  diagnostic is unsettled — ADR 0180 § 11.
- `Response::tls()`, `connectTo` and `redirectToHttp` are stage 11 — `docs/agent/loop-goal.md`
  § *Stage 11*.
- The stage-10 `[context]` overlay was missing the rule the stage executes and both ADR 0180
  sections; this session added them to `docs/agent/loop-goal.toml`.
