# Handoff

## State

**Goal `http-client` — a program talks to a real API. Stages 1–10 are on disk and stage 11's first two
items are landed plus the reporting half of its third: a session now says what it negotiated and a reply
carries it, so the driver's `11 address downgrade report` unit check has all eight of its cases.**
[ADR 0180](../decisions/0180.md) § 13 is the record, and what is left of it is the language surface.

`nvs_host::tls::Session` is the snapshot — `version()` as `TLSv1.3`, `cipher()` under its IANA name,
`chain()` as DER leaf first — read off the completed handshake by `NvsTls::session()`
(`crates/nvs-host/src/tls.rs:287`), on the generic impl rather than beside `peer_addr` because the module
doc keeps `NvsTls<NvsTcp>`'s own pair for what belongs to the socket. `CallPolicy::verifies()`
(`crates/nvs-host/src/tls.rs:735`) is the other half: `anchors` and `min_version` still check the chain
and the name, and the other three each drop one.

`transport::Connection` grew `tls()` (`crates/nvs-stdlib/src/http/transport.rs:402`), asked in `exchange`
at the last line the connection is still the typed thing it was opened as, and `Reply`/`Streamed` each
carry `Option<Tls>`. Nothing reads those two yet, so both wear
`#[cfg_attr(not(test), expect(dead_code, …))]`, which warns the moment the next item reads them.
Nothing is blocked.

## Next group

**Stage 11: the class that answers the report** — one file set: `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/transport.rs`, `tests/conformance/core/`.

- [ ] **`Core\Http\TlsInfo` is the class, and `Response::tls()` the member that answers it** —
      `rule:http-server/a-reply-reports-its-tls-session`, ADR 0180 § 13, which is the only home of the
      seven members and of which parser reads the leaf. A new `CoreClass` beside `RESPONSE`
      (`crates/nvs-stdlib/src/http.rs:1353`), a fourth slot past its three
      (`crates/nvs-stdlib/src/http.rs:1412`), and `exchanged`'s tuple
      (`crates/nvs-stdlib/src/http.rs:2836`) widened to hand `transport::Reply::tls`
      (`crates/nvs-stdlib/src/http/transport.rs:330`) to `request`
      (`crates/nvs-stdlib/src/http.rs:2807`) and to the streamed half. `peerChain` is PEM from the DER
      `Session::chain` holds; `subject`, `issuer` and `expiry` are `x509-parser`'s, already a dependency
      of `nvs-host` for `tlsPin`, so the leaf is parsed there and not in this crate. Reading
      `Reply::tls` and `Tls`'s two fields deletes the `expect(dead_code)` on each
      (`crates/nvs-stdlib/src/http/transport.rs:330`, `crates/nvs-stdlib/src/http/transport.rs:359`) —
      `expect` warns while one is still there.
- [ ] **The cases** — `rule:http-server/a-reply-reports-its-tls-session`. Three `.nvst` cases under
      `tests/conformance/core/`, each calling every `TlsInfo` member so the coverage floor is met per
      member by three files rather than by twenty-one, plus
      `tests/conformance/core/http-response-tls-is-null-for-a-reply-the-table-answered.nvst`, which the
      driver's second stage-11 check names by path and which
      `rule:testing/an-outbound-call-is-answered-from-a-table` is the other half of. The table path is
      `faked` (`crates/nvs-stdlib/src/http.rs:2845`), which builds a response with no session at all.

## Backlog

- `[context] modules` names `nvs-stdlib/src/http.rs` and `http/transport.rs` but not
  `crates/nvs-stdlib/src/http/pool.rs`, whose test module holds a `Connection` impl that a new trait
  method breaks — `docs/agent/loop-goal.toml`.
- Stage 12 onward of `docs/agent/loop-goal.md`, untouched.
