# Handoff

## State

Goal `core-http-response-and-1-more` (feature proofs for `Core\Http\Response` and `Core\Http\TlsInfo`, 14 members).
All seven `Core\Http\Response` members have their feature proofs on disk and measured; `tls` landed last, with a Rust
test that builds a `Core\Http\TlsInfo` straight into the reply's slot (`tls_info_for_example_host` in
`crates/nvs-stdlib/src/http.rs`'s tests, one `rcgen` certificate for `api.example.com`). The seven `TlsInfo`
readers remain. The user decided how a Novis program holds a session offline, and it landed as ADR 0217:
`Core\Test::tlsSession({version?, cipher?, verified?, subject?, issuer?, expiry?}): Core\Http\TlsInfo` builds one
over a real leaf and CA chain, and `Core\Test::answerHttp`'s `tls` option makes a faked reply report it. Every
reader's examples, bench and attack can now run from Novis.
`tests/conformance/core/http-response-tls-reports-the-session-a-test-described.nvst` already reads all seven.

## Next group

**Stage 1: the `Core\Http\TlsInfo` readers** — one file set: `crates/nvs-stdlib/src/http.rs` (readers, tests),
`docs/examples/core/Http-TlsInfo/`, `tests/hostile/core/Http-TlsInfo/`, `benches/members/core/Http-TlsInfo/`. A
program gets its instance with `Core\Test::tlsSession(...)`, directly or through `answerHttp(..., {tls: $session})`
and `Core\Http\Client::get(...)->tls()`. `Core\Test::tlsSession`'s own feature proofs belong to goal
`core-test-2-2`, not here.

- [ ] **`Core\Http\TlsInfo::version` and `::cipher`** — `rule:testing/feature-proofs`; class
      `crates/nvs-stdlib/src/http.rs:1910`, readers from `crates/nvs-stdlib/src/http.rs:4575`. The Rust test builds
      its instance with `tls_info_for_example_host` at `crates/nvs-stdlib/src/http.rs:6337`.
- [ ] **`Core\Http\TlsInfo::subject`, `::issuer`, `::expiry`, `::verified`, `::peerChain`** —
      `rule:testing/feature-proofs`; readers from `crates/nvs-stdlib/src/http.rs:4643`. `subject`, `issuer` and
      `expiry` parse the leaf, so the `rcgen` helper's certificate is what their Rust tests read.

## Backlog

- (nothing yet)
