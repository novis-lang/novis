# Handoff

## State

Goal `core-http-response-and-1-more` (feature proofs for `Core\Http\Response` and `Core\Http\TlsInfo`, 14 members).
All seven `Core\Http\Response` members are complete. `Response::headers` now credits its two `.nvst` cases with a
`covers:` marker, which closed the acceptance check that failed. `TlsInfo::version`, `::cipher`, `::verified`,
`::subject`, `::issuer` and `::expiry` are complete, each with `about.md`, three examples, a bench, an attack and a
Rust test. Only `TlsInfo::peerChain` still owes proofs. **The goal is not DONE after `peerChain`**: the
`subject`/`expiry` proofs found two bugs, items 2 and 3 below, and each is fixed or recorded as a `# Known gaps`
item before the status line says `DONE`.

## Next group

**Stage 1: the `Core\Http\TlsInfo` readers** — one file set: `crates/nvs-stdlib/src/http.rs` (readers, tests),
`crates/nvs-host/src/tls.rs` (`leaf`), `docs/examples/core/Http-TlsInfo/`, `tests/hostile/core/Http-TlsInfo/`,
`benches/members/core/Http-TlsInfo/`. The landed `subject`, `issuer` and `expiry` trees are the model to copy.

- [ ] **`Core\Http\TlsInfo::peerChain`** — `rule:testing/feature-proofs`; reader at
      `crates/nvs-stdlib/src/http.rs:4675`. It returns the chain as DER `bytes`, leaf first. The Rust test can
      build a described session with `nvs_host::tls::described` and `super::tls_info_of`, as
      `tls_info_subject_issuer_and_expiry_read_the_leaf_of_the_chain` does; a described chain has 2 entries.
- [ ] **A subject or issuer value is not escaped, so two names read back as the same text** —
      `rule:testing/a-failing-proof-is-fixed-or-recorded`; `crates/nvs-host/src/tls.rs:473` uses
      `X509Name::to_string`, which writes neither RFC 4514's `\+` nor `\,`, although `Leaf::subject`'s doc says
      RFC 4514. `Core\Test::tlsSession({subject: "CN=x+O=y"})->subject()` returns `CN=x+O=y`, the same text a
      two-attribute name gives. Fix: format each RDN over `iter_rdn()` and escape the value per RFC 4514 § 2.4,
      keeping the `, ` joiner the `.nvst` cases expect, and pin it with a `.nvst` case.
- [ ] **`TlsInfo::expiry` is a fatal for a peer whose `notAfter` is `99991231235959Z`** —
      `rule:testing/a-failing-proof-is-fixed-or-recorded`; `crates/nvs-stdlib/src/http.rs:4733` calls that
      branch unreachable, but `Core\Time\Instant` ends at `9999-12-30T22:00:00Z`, and RFC 5280 § 4.1.2.5 names
      exactly that date for a certificate with no end. `tlsSession` cannot build one, so only a Rust test with
      an `rcgen` leaf reaches it. Deciding what it returns (the last instant, or an error a program can catch)
      is a design call: record it as a `# Known gaps` item in `crates/nvs-stdlib/src/http.rs`'s module doc with
      an owner unless one sentence of an ADR already decides it.

## Backlog

- `TlsInfo::subject` and `::issuer` parse the whole leaf on every call: 23 allocations and about 2 KB per read
  (`docs/perf/members.ndjson`). Parsing once when the session is built would make both a slot read.
- `Core\Test::tlsSession`'s own feature proofs belong to goal `core-test-2-2`.
- `crates/nvs-cli/src/script.rs:1709` `a_revalidation_that_wins_publishes_and_readers_never_block_on_a_compile`
  failed once beside the other test binaries and passed alone; it leans on timing that load breaks.
