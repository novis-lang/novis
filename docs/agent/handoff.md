# Handoff

## State

Goal `core-http-response-and-1-more` (feature proofs for `Core\Http\Response` and `Core\Http\TlsInfo`, 14 members).
All seven `Core\Http\Response` members have their feature proofs on disk and measured; `tls` landed last, with a Rust
test that builds a `Core\Http\TlsInfo` straight into the reply's slot (`tls_info_for_example_host` in
`crates/nvs-stdlib/src/http.rs`'s tests, one `rcgen` certificate for `api.example.com`). The seven `TlsInfo`
readers remain, and **they are blocked on a user decision**: no Novis program can hold a non-null `TlsInfo` without a
network. `Core\Test::answerHttp` answers with no session (`tls()` is `null`), `Core\Test::serverUrl` is `http` only,
and no Core member serves TLS. So a Novis test, three examples, a bench and an attack for `version`, `cipher`,
`verified`, `peerChain`, `subject`, `issuer` and `expiry` cannot run their member at all. The Rust half can.

## Next group

**Stage 1: the `Core\Http\TlsInfo` readers, after the decision** — one file set: `crates/nvs-stdlib/src/http.rs`
(readers, tests), `crates/nvs-stdlib/src/test.rs` (if `answerHttp` grows), `docs/examples/core/Http-TlsInfo/`,
`tests/hostile/core/Http-TlsInfo/`, `benches/members/core/Http-TlsInfo/`.

- [ ] **The user's decision: how a Novis program gets a TLS session offline** — `rule:testing/feature-proofs`;
      `Core\Test::answerHttp` at `crates/nvs-stdlib/src/test.rs:2066`. (a) `answerHttp` takes a `tls` option (version,
      cipher, verified, a PEM chain) and the faked reply carries that session. It is test-only, costs nothing at
      runtime, and needs a numbered ADR, which this goal may not open. (b) A `[skip]` in
      `tools/data/dossier-policy.toml` for the examples, bench and attack of all seven readers, with Rust tests only.
      (a) is the recommendation: (b) leaves seven members with no example a reader can learn from.
- [ ] **`Core\Http\TlsInfo::version` and `::cipher`** — `rule:testing/feature-proofs`; class
      `crates/nvs-stdlib/src/http.rs:1910`, readers from `crates/nvs-stdlib/src/http.rs:4575`. The Rust test builds
      its instance with `tls_info_for_example_host` at `crates/nvs-stdlib/src/http.rs:6337`.
- [ ] **`Core\Http\TlsInfo::subject`, `::issuer`, `::expiry`, `::verified`, `::peerChain`** —
      `rule:testing/feature-proofs`; readers from `crates/nvs-stdlib/src/http.rs:4643`. `subject`, `issuer` and
      `expiry` parse the leaf, so the `rcgen` helper's certificate is what their Rust tests read.

## Backlog

- `tests/conformance/core/http-response-tls-is-null-for-a-reply-the-table-answered.nvst` has a `TlsInfo` branch that
  never runs; once a faked session exists, a twin case with a session pins the seven readers from Novis.
