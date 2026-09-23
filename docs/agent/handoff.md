# Handoff

## State

Goal `core-http-response-and-1-more` (feature proofs for `Core\Http\Response` and `Core\Http\TlsInfo`, 14 members).
All seven `Core\Http\Response` members are complete, and `TlsInfo::version`, `::cipher` and `::verified` are too:
`about.md`, three examples, a bench, an attack and a Rust test each. `Core\Http\Response` and `Core\Http\TlsInfo`
now carry their class cards (`RESPONSE_CARD`, `TLS_INFO_CARD` in `crates/nvs-stdlib/src/http.rs`), so the help
proof is closed for all fourteen members. Every figure in `docs/perf/members.ndjson` for the ten measured members
is current against `http.rs` as committed. Four `TlsInfo` readers remain. A program gets its instance with
`Core\Test::tlsSession({...})`, directly or through `Core\Test::answerHttp(..., {tls: $session})` and
`Core\Http\Client::get(...)->tls()`. `tests/conformance/core/http-response-tls-reports-the-session-a-test-described.nvst`
already reads all seven, so each remaining reader owes one Rust test.

## Next group

**Stage 1: the `Core\Http\TlsInfo` readers** — one file set: `crates/nvs-stdlib/src/http.rs` (readers, tests),
`docs/examples/core/Http-TlsInfo/`, `tests/hostile/core/Http-TlsInfo/`, `benches/members/core/Http-TlsInfo/`. The
landed `version`, `cipher` and `verified` trees are the model to copy. `Core\Test::tlsSession`'s own feature
proofs belong to goal `core-test-2-2`, not here.

- [ ] **`Core\Http\TlsInfo::subject`, `::issuer` and `::expiry`** — `rule:testing/feature-proofs`; readers from
      `crates/nvs-stdlib/src/http.rs:4696`, cards from `crates/nvs-stdlib/src/http.rs:2046`. All three parse the
      leaf of the chain. Their Rust tests can build a session with `nvs_host::tls::described` and
      `super::tls_info_of`, the way `crates/nvs-stdlib/src/http.rs:6551` does. `tlsSession` takes `subject` and
      `issuer` as `KEY=value` pairs (`CN`, `O`, `OU`, `C`, `ST`, `L`), and `expiry` as a `Core\Time\Instant` in
      whole seconds.
- [ ] **`Core\Http\TlsInfo::peerChain`** — `rule:testing/feature-proofs`; reader at
      `crates/nvs-stdlib/src/http.rs:4675`. It returns `array<tainted string>` of DER certificates, leaf first, and
      a described session has two. Measure every bench with one `dossier.py --record-perf --only …` after the last
      `http.rs` edit and `cargo fmt`, since any edit to that file makes all its figures stale.

## Backlog

- `nvs agent show 'Core\Http\TlsInfo'` does not print a class card's `short` yet — goal `core-class-cards`.
- The Rust fixture `tls_info_for_example_host` fills the cipher slot with `TLS13_AES_128_GCM_SHA256`, which is not
  the IANA name `nvs_host::tls` writes; harmless for its one reader, and worth aligning when that test is next
  touched (`crates/nvs-stdlib/src/http.rs:6391`).
