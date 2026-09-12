# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–6 are on disk, and stage 7 now holds its pool half and the buffered half of
its compression half.** [ADR 0180](../decisions/0180.md) is the record and the home of every
decision this goal executes.

Every request head writes `Accept-Encoding: gzip, br, zstd` — `OFFERED` in
`crates/nvs-stdlib/src/http/transport.rs` — and an `Accept-Encoding` a program wrote is **refused**
naming it rather than sent beside ours: an offer is a promise to decode, so the set is the client's
and a program can neither widen nor narrow it. That is this session's one tradeoff, and it costs a
program the ability to ask an origin for `identity`. A buffered reply under one of the three is
decoded in `send`, under `Call::compress` — `compress::Bound::ceiling(ctx)`, carried like
`Call::pool` — with `REPLY_CEILING` lowered onto it, and what comes back carries neither
`Content-Encoding` nor `Content-Length`. `decompress_within` now takes the member a refusal names,
so an HTTP reply's refusal no longer says `Core\Compress::decompress()`. A zstd frame's declared
window is capped at 8 MiB before the decoder allocates it, which `Bound` cannot reach and
`ruzstd` defaults to 100 MiB.

Stage 7 still owes the streamed half and § 7's client identity, which is also part of the pool key.
Nothing is blocked.

## Next group

**Stage 7: a reply may arrive compressed, streamed** — one file set:
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/http/stream.rs`,
`crates/nvs-stdlib/src/compress.rs`.

- [ ] **A streamed reply is decoded as it arrives, and its decoder's window is charged to the
      request** — the coding is the head's and `coding_of`
      (`crates/nvs-stdlib/src/http/transport.rs:835`) already reads it, so `send_streamed`
      (`crates/nvs-stdlib/src/http/transport.rs:885`) is where it is decided and
      `Incoming::pull` (`crates/nvs-stdlib/src/http/transport.rs:531`) is where the octets are
      framed and would be decoded, one read at a time rather than gathered first. The decoder is a
      `Read` over a source that is not all here yet, which is the shape `decompress_within`
      (`crates/nvs-stdlib/src/compress.rs:539`) does not have — it takes a whole slice.
      `rule:core-classes/decompression-bound`, and the goal's § *Standing decisions* fixes the
      windows: 32 KiB for gzip, 8 MiB for zstd, 16 MiB for brotli, charged to the request and
      released with the stream.
- [ ] **A client identity is presented when asked and is part of the pool key** — `pool_key`
      (`crates/nvs-stdlib/src/http/transport.rs:1029`) is what a reuse agrees on and the identity
      has to reach it, and the option arrives where the `Call` is built
      (`crates/nvs-stdlib/src/http.rs:2101`). A call without an identity never reuses a connection
      that presented one. `rule:security/db-pool-reset-is-a-boundary` is the shape — every
      credential is part of the key.

## Backlog

- An armed answer table (`rule:testing/an-outbound-call-is-answered-from-a-table`) hands its octets
  over without a decode, so a case arming a `Content-Encoding` reply gets the frame — decide in
  [ADR 0180](../decisions/0180.md) whether a table's octets are already decoded.
- Brotli's window is whatever the frame declares; only zstd's is bounded
  (`crates/nvs-stdlib/src/compress.rs`'s `ZSTD_WINDOW`). The standing decision names 16 MiB for it.
- A program cannot ask an origin for `identity`, since `Accept-Encoding` is refused to it
  ([ADR 0180](../decisions/0180.md)).
- `Core\Compress::decompress` still offers `Zlib` and `Deflate`, which no HTTP reply may use —
  the two sets are deliberately different (`docs/rules/core-classes/decompression-bound.md`).
