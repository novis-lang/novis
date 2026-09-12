# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–6 are on disk, and stage 7 now holds its pool half and the whole of its
compression half.** [ADR 0180](../decisions/0180.md) is the record and the home of every decision
this goal executes.

A streamed reply under one of the three offered codings is decoded **as it arrives**.
`transport::Incoming` is now the framing (`Framed`) with a decoder optionally in front of it
(`Coding::As` / `Coding::Under`), so the walks in `http/stream.rs` read decoded octets from either
and never learn which. `send_streamed` decides the coding from the head and strips
`Content-Encoding` and `Content-Length`, exactly as the buffered `send` does. The decode is
`compress::Decoder` — each backend's own `Read` over a source that blocks, built at the first read
because a zstd decoder reads its frame header as it is constructed. What bounds it is its window
(32 KiB DEFLATE, 8 MiB zstd, 16 MiB brotli with the large-window extension off) plus one
`transport::STEP`, because a stream has no total for an output ceiling to be over; the wait is
bounded by `idle` and `maxDuration` as before. A framing refusal travels out past the decoder in
`Incoming::faulted`, so a silence is still a `TimeoutError` and not a `ParseError` about zstd.

Stage 7 still owes § 7's client identity, which is also part of the pool key. Nothing is blocked.

## Next group

**Stage 7: a client identity is presented when asked and is part of the pool key** — one file set:
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/crypto.rs`,
`crates/nvs-host/src/tls.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **`Core\Http\Identity` reads a PEM chain over a key pair and refuses a leaf that does not
      match it** — a new Tier 0 class beside `Core\Crypto\KeyPair`
      (`crates/nvs-stdlib/src/crypto.rs:352`, whose `KEY_PAIR_NAME` is the spelling to follow), with
      one static `read(bytes $chainPem, Crypto\KeyPair $key)`. The refusal is a leaf whose public key
      is not the pair's, which is the only check `read` can make locally. Its registry row and doc
      card go beside the option rows at `crates/nvs-stdlib/src/http.rs:351`.
      `rule:core-api/shape-rules`, and the goal's § *Stage 7* prose is what it executes.
- [ ] **`identity?: Core\Http\Identity` joins the bag and reaches the handshake** — a new option
      constant beside `RETRY_KEY_OPTION` (`crates/nvs-stdlib/src/http.rs:353`) and a new slot beside
      `RETRY_KEY` (`crates/nvs-stdlib/src/http.rs:814`), carried on `Call` the way `Call::pool` and
      `Call::compress` are. One `ClientConfig` per `Identity`, built once and held as long as it is,
      beside the process-wide one at `crates/nvs-host/src/tls.rs:314` — that module doc's
      "what that spends" paragraph (`crates/nvs-host/src/tls.rs:85`) is where the second config's
      cost is stated. `rule:security/db-pool-reset-is-a-boundary` is the shape the key follows.
- [ ] **The pool key includes the identity** — `pool_key`
      (`crates/nvs-stdlib/src/http/transport.rs:1309`) gains it, so two identities never share a
      connection and a call without one never reuses a connection that presented one. The two Rust
      tests the stage's `[[check]]` names are
      `client_identity_is_presented_when_asked_and_is_part_of_the_pool_key` and
      `a_call_without_an_identity_never_reuses_a_connection_that_presented_one`, and the `.nvst` case
      is `tests/conformance/core/http-identity-reads-a-pem-chain-and-a-key-pair-and-refuses-a-mismatch.nvst`.
      `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`.

## Backlog

- Stage 8 onward is untouched: credentials on a redirect, then the trust-root and grant stages.
- `[context] modules` printed no entry for `crates/nvs-host/src/tls.rs`'s client-auth half; the next
  group needs it and the field should name it.
- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details stay the `nvs/rest`
  package's, per the goal's § *Standing decisions*.
