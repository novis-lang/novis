# Handoff

## State

**Goal `http-client` — a program talks to a real API. Stages 1–10 are on disk and stage 11 is two
thirds landed: a call may name the address it connects to and is still judged for it, and an `https`
hop down into plaintext needs both the `net.downgrade` grant and `redirectToHttp: true`.**
[ADR 0180](../decisions/0180.md) § 12 is the record those two execute, and both its bullets are done.

The bag now ends `tlsMinVersion`, `connectTo`, `redirectToHttp` at slots 18–20, with `stream`'s own two
bounds after them at 21 and 22 (`crates/nvs-stdlib/src/http.rs:1166`). `connectTo` goes through
`named_address` (`crates/nvs-stdlib/src/http.rs:2309`), which asks `net.connect` and `net.connect_to` of
the **URL's** host and then hands the literal to `nvs_runtime::capability::pinned_address`, so
`rule:security/net-address-policy` keeps its one home and the certificate is still checked against the
name the URL wrote. Beside a `Core\Http\Target` the key is a `LogicError`.

The downgrade is split across the two modules because neither half can decide alone: `transport::sent`
(`crates/nvs-stdlib/src/http/transport.rs:1051`) is the only place both schemes are in view, so it
reports the hop and decides nothing, and `http::repinned` (`crates/nvs-stdlib/src/http.rs:2344`) holds
the `Ctx` and the call's bag. Every `repin` closure is `FnMut(&str, bool)` as a result.

Stage 11's third item is untouched, and `nvs_host::tls::NvsTls` reports nothing about the session it
negotiated yet, so that is where it starts. Nothing is blocked.

## Next group

**Stage 11: the reply reports its TLS session** — one file set: `crates/nvs-host/src/tls.rs`,
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/http.rs`, `tests/conformance/core/`.

- [ ] **`NvsTls` reports the session it negotiated, and a reply carries it** —
      `rule:http-server/a-reply-reports-its-tls-session`, ADR 0180 § 13, which is the only home of
      which parser reads the leaf's subject, issuer and expiry: a reporting accessor beside
      `NvsTls::peer_addr` (`crates/nvs-host/src/tls.rs:445`) answering the protocol version, the
      negotiated cipher suite and the peer chain as DER, a field on `transport::Reply`
      (`crates/nvs-stdlib/src/http/transport.rs:330`) filled where the reply is framed
      (`crates/nvs-stdlib/src/http/transport.rs:1442`) and `None` for a plaintext `Connection`
      (`crates/nvs-stdlib/src/http/transport.rs:352`), and `verified` false wherever `Call::policy`
      relaxed anything (`crates/nvs-stdlib/src/http/transport.rs:199`). The two unit cases the
      driver's check names — `tls_info_reports_version_cipher_and_the_peer_chain` and
      `tls_info_is_unverified_after_a_pinned_or_relaxed_call` — belong here, beside
      `https_call_through_the_client_reaches_a_loopback_origin_under_a_roots_file`, which already
      spins a TLS origin.
- [ ] **`Core\Http\TlsInfo` is the class, and `Response::tls()` the member that answers it** —
      `rule:http-server/a-reply-reports-its-tls-session`, the five edits of conventions.md § *A `Core`
      member*: the class beside `RESPONSE` (`crates/nvs-stdlib/src/http.rs:1353`), a fourth response
      slot after `HEADERS_SLOT` (`crates/nvs-stdlib/src/http.rs:1418`), `version()`, `cipher()`,
      `verified()`, `peerChain(): array<tainted string>` as PEM and the leaf's `subject()`, `issuer()`
      and `expiry()`, and `null` out of the faked arm (`crates/nvs-stdlib/src/http.rs:3106`), which is
      the second acceptance check's whole subject.
- [ ] **The cases** — `rule:http-server/a-reply-reports-its-tls-session`:
      `tests/conformance/core/http-response-tls-is-null-for-a-reply-the-table-answered.nvst`, which the
      driver's second stage-11 check names by path, and the three-per-member floor
      `crates/nvs-stdlib/tests/conformance_coverage.rs:155` enforces for every member the slice above
      adds — the four shapes a depth case takes are conventions.md § *A `.nvst` test case*, and the
      class the cases call is `crates/nvs-stdlib/src/http.rs:1353`'s neighbour.

## Backlog

- Stage 12 — a name resolved off the core, and every address it answers — shares this file set and can
  follow in the same group (`docs/agent/loop-goal.md:308`).
- `[context.stage.12]` in `docs/agent/loop-goal.toml` names no `adrs`, as stage 11's did not until this
  session; check it before the group opens rather than from inside it.
- A redirect hop never inherits `connectTo`: the hop re-resolves through `pin`, which is the safe
  reading and is not written down anywhere — `rule:http-server/an-outbound-call-names-its-address-only-under-a-grant`
  is where it would go if stage 14 wants it stated.
