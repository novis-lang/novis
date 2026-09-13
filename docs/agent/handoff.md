# Handoff

## State

**Goal `http-client` — a program talks to a real API. Stages 1–9 are on disk; stage 10's
configuration half, its option half and now its verifiers are landed, and nothing reads a
relaxing option after the judging yet.** [ADR 0180](../decisions/0180.md) § 11 is the record this
stage executes.

`nvs_host::tls` is the one client and it now answers all four relaxations. `CallPolicy`
(`crates/nvs-host/src/tls.rs:609`) is one field per option — `anchors`, `pins`, `any_name`,
`insecure`, `min_version` — and `config_for` (`crates/nvs-host/src/tls.rs:908`) turns it into a
session: `PresentedKey` for the two that build no chain, `AnyName` for the one that builds a chain
and skips the name, and the shipped builder otherwise. `NvsTls::over_policy`
(`crates/nvs-host/src/tls.rs:279`) is the only door, it takes the policy value and an optional
`&NvsIdentity` and never a session (`rule:security/one-tls-client`), and a default policy hands
back the process's own `Arc` untouched. `NvsIdentity` keeps its chain and key as well as its built
configuration, so mutual TLS against a private CA is one session rather than a choice between the
two halves. `CONFIGURED` holds the anchors a boot resolved, because a call that only raises its
floor still builds its chain against the operator's set.

Nothing on the stdlib side has changed: `judge_trust` still asks the grants and then drops the
options on the floor, so `Call`, `pool_key` and the handshake site are where stage 10's `nvs-stdlib`
check still fails. That is an open item, not a regression.

Nothing is blocked.

## Next group

**Stage 10: the policy through the transport, and the boot's own line** — one file set:
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **The policy is built where the options are judged, and travels on the call** — the five keys
      that `judge_trust` (`crates/nvs-stdlib/src/http.rs:1923`) has already proved a grant for
      become one `nvs_host::tls::CallPolicy`, on a new `Call` field beside `identity`
      (`crates/nvs-stdlib/src/http/transport.rs:194`), and the handshake takes it:
      `NvsTls::over_policy(stream, &parts.host, policy, identity)` replaces both arms at
      `crates/nvs-stdlib/src/http/transport.rs:1294`. `tls_min_version_below_the_floor_throws` is
      the one of stage 10's four `nvs-stdlib` tests that belongs here — the floor is
      `[http.client.tls] min_version` and the throw is a `LogicError`, per
      `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`.
- [ ] **A relaxed connection never serves a strict call** — the policy joins `pool_key`
      (`crates/nvs-stdlib/src/http/transport.rs:1349`), which is a `String` of
      `scheme|host|socket|identity` today and needs one more field that is empty for a call
      relaxing nothing, the shape `identity` already has. `CallPolicy` is `Eq + Hash` for exactly
      this. `a_relaxed_connection_never_serves_a_strict_call_from_the_pool` goes beside
      `a_call_without_an_identity_never_reuses_a_connection_that_presented_one`
      (`crates/nvs-stdlib/src/http/transport.rs:3466`), which is its twin.
- [ ] **The boot prints every relaxed grant, one line per grant and host** — ADR 0180 § 11's last
      bullet, where the boot already settles the outbound client from the snapshot
      (`crates/nvs-cli/src/config.rs:349`, the policy it hands over at
      `crates/nvs-cli/src/config.rs:776`). A weakening nobody is reminded of outlives the incident
      it was added for. This one is `crates/nvs-cli/src/config.rs` rather than the file set above.

## Backlog

- `a_tls_option_without_its_host_grant_throws_before_connecting` and
  `a_tls_grant_for_another_host_relaxes_nothing` are stage 10's other two `nvs-stdlib` tests; they
  assert `judge_trust`, which is already on disk, so they are a test-writing slice over
  `crates/nvs-stdlib/src/http.rs` alone.
- `a_tls_grant_of_true_is_refused` (`nvs-config`) is stage 10's third check — check whether the
  configuration half already refuses it before writing anything.
- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details are the `nvs/rest`
  package's, not this goal's (goal `http-client` § *Standing decisions*).
