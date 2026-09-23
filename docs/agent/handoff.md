# Handoff

## State

Goal `core-http-response-and-1-more` (feature proofs for `Core\Http\Response` and `Core\Http\TlsInfo`, 14 members).
`Core\Http\Response::bytes`, `::header`, `::headers`, `::text`, `::status` and `::jsonAs` have all their feature
proofs on disk and measured. Their examples use `Core\Test::answerHttp` for a fixed reply; the Rust tests share
`answered_once` in `crates/nvs-stdlib/src/http.rs`'s tests, and `named_shape` there builds the `{name: string}`
shape a compiled `jsonAs<T>()` call site hands the helper. Nothing is blocked. Eight members remain: `tls` and the
seven `TlsInfo` readers.

## Next group

**Stage 1: `Core\Http\Response::tls` and the `Core\Http\TlsInfo` readers** — one file set: `crates/nvs-stdlib/src/http.rs`
(registry rows, helpers, tests), `docs/examples/core/Http-Response/tls/`, `docs/examples/core/Http-TlsInfo/`,
`tests/hostile/core/`, `benches/members/core/`. `tls()` is `null` for a reply `Core\Test::answerHttp` answered
(`tests/conformance/core/http-response-tls-is-null-for-a-reply-the-table-answered.nvst`), so a non-null example needs
either a real TLS origin or a way for the table to carry a session — read the `answerHttp` row first and decide.

- [ ] **`Core\Http\Response::tls`** — `rule:testing/feature-proofs`; row `crates/nvs-stdlib/src/http.rs:1746`. The `null` answer is pinned from Novis; a Rust test needs a TLS loopback origin or the slot written directly.
- [ ] **`Core\Http\TlsInfo::version` and `::cipher`** — `rule:testing/feature-proofs`; class `crates/nvs-stdlib/src/http.rs:1910`, rows `crates/nvs-stdlib/src/http.rs:1916` and `crates/nvs-stdlib/src/http.rs:1925`. A Rust test can build the instance with `crate::instance::build(&TLS_INFO, [...])`.
- [ ] **`Core\Http\TlsInfo::subject`, `::issuer` and the other three readers** — `rule:testing/feature-proofs`; rows from `crates/nvs-stdlib/src/http.rs:1952`.

## Backlog

- A shape's internal label leaks into messages: `1 field(s) of `$shape{id,name}` did not match` (`crates/nvs-stdlib/src/json.rs:2454` and about twenty sibling sites, plus `crates/nvs-stdlib/src/db/row.rs`). Five `tests/conformance/lang/` cases and `docs/examples/lang/errors/properties-not-accessors/02-every-problem-in-one-list.out` pin the spelling, so changing it is a tree-wide call, not a proof slice.
- `Core\Http\Response::jsonAs` spends 12 allocations and 566 bytes to decode a two-field shape (`docs/perf/members.ndjson`), a perf opportunity in `crates/nvs-stdlib/src/json.rs`'s `decode_as`.
- `benches/members/core/Http-Response/bytes.nvs` chains on `$total % 2` over two even-length bodies, so it only reads the first reply; the `text`, `status` and `jsonAs` benches alternate. Fixing it re-measures `bytes`.
