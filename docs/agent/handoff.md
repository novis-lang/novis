# Handoff

## State

Goal `core-http-response-and-1-more` is reached. All 14 members of `Core\Http\Response` and `Core\Http\TlsInfo`
have their feature proofs, and `dossier.py --verify --group` is green for both classes. The two bugs the
`subject`/`expiry` proofs found are closed: a certificate name's value now reads back escaped per RFC 4514 § 2.4
(`crates/nvs-host/src/tls.rs`'s `written`), and the `expiry` fatal for a `notAfter` of `99991231235959Z` is
recorded as `crates/nvs-stdlib/src/http.rs`'s first `# Known gaps` item, owner M10.

## Next group

**Stage 1: whatever the next goal names** — the driver switches goals on this `DONE`, and the switch overwrites
this file with the next goal's own handoff. One item this goal leaves behind, owned by M10 rather than by a goal:

- [ ] **`TlsInfo::expiry` has no instant for `99991231235959Z`** — `rule:testing/a-failing-proof-is-fixed-or-recorded`;
      the fatal at `crates/nvs-stdlib/src/http.rs:4743`. Returning the last instant or a catchable error is a design
      call, and a Rust test with an `rcgen` leaf is the only way to reach it.

## Backlog

- `TlsInfo::subject`, `::issuer` and `::expiry` parse the whole leaf on every call (about 23 allocations and 2 KB
  per read in `docs/perf/members.ndjson`); `peerChain` base64-encodes every certificate on every call (19
  allocations, 5.8 KB). Parsing and encoding once when the session is built would make each a slot read.
- `tests/hostile/core/Http-Response/text/01-large-and-broken-text-bodies.nvs` takes 7 to 9 s under release, most
  of it `Core\Str::length` over 32 million characters; it now declares a 30 s timeout.
- `Core\Test::tlsSession` parses a name by splitting on `,`, so a value with an escaped `\,` cannot be written;
  its own feature proofs belong to goal `core-test-2-2`.
