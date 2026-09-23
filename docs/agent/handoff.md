# Handoff

## State

Goal `core-http-response-and-1-more` (feature proofs for `Core\Http\Response` and `Core\Http\TlsInfo`, 14 members).
`Core\Http\Response::bytes`, `::header` and `::headers` have all their feature proofs on disk and measured.
Their examples use `Core\Test::answerHttp` for a fixed reply, so they show a real `Core\Http\Response`
without a network; the Rust tests share `answered_once` in `crates/nvs-stdlib/src/http.rs`'s tests, a
loopback origin that hands back the response itself. `header`/`headers` now share the stored header
strings rather than copying each line (5 allocations per `header` read became 1).
Nothing is blocked. Eleven members remain: `jsonAs`, `status`, `text`, `tls` and the seven `TlsInfo` readers.

## Next group

**Stage 1: `Core\Http\Response` body readers** — one file set: `crates/nvs-stdlib/src/http.rs` (registry rows, helpers, tests),
`docs/examples/core/Http-Response/`, `tests/hostile/core/Http-Response/`, `benches/members/core/Http-Response/`.
Reuse `answered_once` for the Rust half and `Core\Test::answerHttp` for the examples, as the `bytes` proofs do.

- [ ] **`Core\Http\Response::text`** — `rule:testing/feature-proofs`; the helper is `crates/nvs-stdlib/src/http.rs:4300`, its row `crates/nvs-stdlib/src/http.rs:1700`. The not-UTF-8 refusal is already pinned from Rust by `response_bytes_reads_a_body_text_refuses_and_reads_it_again`; its own test should pin a valid body and the byte offset the refusal names.
- [ ] **`Core\Http\Response::status`** — `rule:testing/feature-proofs`; row `crates/nvs-stdlib/src/http.rs:1691`. A `404`/`500` is an answer, not a throw (STATUS_DOC).
- [ ] **`Core\Http\Response::jsonAs`** — `rule:testing/feature-proofs`; helper `crates/nvs-stdlib/src/http.rs:4397`, row `crates/nvs-stdlib/src/http.rs:1718`. Its examples need a `#[Json\Derive]` class whose text fields are `tainted`.

## Backlog

- `Core\Hash::of` and `::hmac` take `CoreTy::Bytes`/`CoreTy::Str` (`crates/nvs-stdlib/src/hash.rs:298`), so a downloaded `tainted bytes` body cannot be checked against a published SHA-256 checksum without a launderer. Possibly `CoreTy::Blob(Qual::Neutral)`; a security call, not taken here — owning doc `crates/nvs-stdlib/src/hash.rs`.
- The `TlsInfo` readers (`crates/nvs-stdlib/src/http.rs:1907`-`1961`) need a real TLS session, since an answered call's `tls()` is `null`; look for the loopback TLS fixture the `Client` tests use before writing one.
