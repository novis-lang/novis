# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–9 are on disk; stage 10's configuration half is landed and its two code
halves are not.** [ADR 0180](../decisions/0180.md) is the record and the home of every decision this
goal executes.

The six grants a relaxing call is owed exist and are on the roster: `[capabilities.tls]`'s
`anchors`, `pin`, `any_name` and `insecure` (`crates/nvs-config/src/tree.rs:@CapTls`), and
`connect_to` and `downgrade` on `CapNet`. `net.connect_to` and `net.downgrade` belong to stage 11's
options and land here because the block they sit in is written once. **None has a `true` spelling**:
`Cap::takes_true_spelling` names the six, and `grant_for` — which every asker in
`crates/nvs-config/src/capability.rs` now goes through — reads a `true` for one of them as nothing
granted, so a deployment that wrote the spelling it knows relaxes nothing until it names hosts.

Nothing else reads those grants yet. `nvs_stdlib`'s bag has no `tlsCa`/`tlsPin`/`tlsVerifyHost`/
`tlsVerify`, and `nvs_host::tls` has no verifier behind them, so stage 10's `nvs-host` and
`nvs-stdlib` checks are still open items rather than regressions.

Nothing is blocked.

## Next group

**Stage 10: the options and the verifiers behind them** — one file set:
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/http/transport.rs`,
`crates/nvs-host/src/tls.rs`.

- [ ] **The four options join the bag** — `tlsCa`, `tlsPin`, `tlsVerifyHost`, `tlsVerify` as
      `CoreOption` rows beside `IDENTITY_OPTION` in the `request_options!` macro
      (`crates/nvs-stdlib/src/http.rs:721`), their key constants beside
      `crates/nvs-stdlib/src/http.rs:782`, and one `ParamDoc` each in the shared card block the
      `identity` entry sits in (`crates/nvs-stdlib/src/http.rs:1523`). `tlsMinVersion` joins them
      and needs no grant, because it can only tighten.
- [ ] **Each option is refused at the call when its grant does not name the URL's host** — asked of
      the request's own snapshot through `Cap::TlsAnchors`/`TlsPin`/`TlsAnyName`/`TlsInsecure`
      (`crates/nvs-config/src/capability.rs:99`) with `allows_host`, throwing a `LogicError` naming
      the grant before a socket is opened, in the transport's send path
      (`crates/nvs-stdlib/src/http/transport.rs`).
      `rule:security/capability-question-is-grant-and-scope` is the shape of the ask.
- [ ] **The verifiers, inside the one client** — a call naming any of those options builds its own
      `ClientConfig` through `rustls`'s custom-verifier seam beside `anchors_from`
      (`crates/nvs-host/src/tls.rs:695`), and a caller still hands in a policy value rather than a
      session (`rule:security/one-tls-client`). The policy is part of stage 7's pool key, so a
      relaxed connection never serves a call that verifies.

## Backlog

- `[http.client.tls] keylog` is not resolved against the file that wrote it, where `roots` is
  (`crates/nvs-config/src/http.rs:342`) — whether `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`
  reaches a file the runtime creates rather than reads is undecided.
- The boot printing every relaxed grant, one line per grant and host — stage 10's last bullet in
  [loop-goal.md](loop-goal.md), and goal `outbound-proxy` does the same for its own weakening.
- `Response::tls(): ?Core\Http\TlsInfo`, `connectTo` and the `https`→`http` downgrade refusal —
  stage 11, whose two grants are already on the roster.
- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details are the `nvs/rest`
  package's, per the goal's § *Standing decisions*.
