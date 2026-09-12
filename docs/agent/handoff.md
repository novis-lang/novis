# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–6 are on disk, and stage 7's pool half is now too: a connection outlives its
call.** [ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal
executes.

`crates/nvs-stdlib/src/http/pool.rs` is the store — a `thread_local!` `Vec` the calling core owns,
under `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`. The key is
`transport::pool_key`'s, built from the pinned address, the port, the scheme and the server name, so
nothing the pool reads can widen reuse past the check the door made. `Incoming::finished` is what
gives a connection back, and only for a head `reusable` accepted, so a body the close itself
delimits, a `Connection: close` and a walk that stopped short each drop the socket. The request head
no longer carries `Connection: close`; `[http.client] pool_idle` and `pool_idle_timeout` are the two
`System`-class caps and reach the transport as `Call::pool`.

Stage 7 still owes every compression item and § 7's client identity, which is also part of the pool
key. Nothing is blocked. One thing is deliberately the record's literal reading rather than the
wider one: a drawn connection that is silent — closed before an octet of the reply — is an attempt
under the retry rules, and only a failure with the request still going out is replaced free.

## Next group

**Stage 7: a reply may arrive compressed** — one file set:
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/compress.rs`,
`crates/nvs-stdlib/src/http.rs`.

- [ ] **The request offers gzip, brotli and zstd, and a buffered reply is decoded under the compress
      bound** — the head writes `Accept: */*` and no `Accept-Encoding`
      (`crates/nvs-stdlib/src/http/transport.rs:1068`), and `decompress_within`
      (`crates/nvs-stdlib/src/compress.rs:522`) is the bound both halves share. `Bound::ceiling`
      takes a `Ctx` this module deliberately cannot reach, so it rides on the call the way
      `Call::pool` does (`crates/nvs-stdlib/src/http/transport.rs:124`). A coding the request did
      not offer is refused naming it. `rule:core-classes/decompression-bound`.
- [ ] **A streamed reply is decoded as it arrives, and its decoder's window is charged to the
      request** — `Incoming` is the reader every walk frames off
      (`crates/nvs-stdlib/src/http/transport.rs:351`) and `whole` is the buffered drain of that same
      reader (`crates/nvs-stdlib/src/http/transport.rs:577`), so a coding undone only in the
      buffered half hands a walk compressed octets. ADR 0180 § 17 is what the window spends.
- [ ] **A zstd frame whose window passes 8 MiB is refused before it is allocated** — the zstd arm
      builds a `ruzstd::decoding::StreamingDecoder` (`crates/nvs-stdlib/src/compress.rs:539`), which
      is past the point where the window is chosen; the frame header carries it ahead of any of it.
      ADR 0180 § 8.

## Backlog

- Stage 7's client identity is a second check (`docs/agent/loop-goal.toml:9485`) and its own group:
  a PEM chain over a key pair, part of the pool key.
- A drawn connection closed before it answered spends an attempt rather than being replaced free —
  ADR 0180 § 6 gives the free replacement to a send that failed and to nothing else.
- A dribbling origin is only reachable from `transport.rs`'s own tests; the walk over a reader is
  pinned at the framing functions rather than end to end — `crates/nvs-stdlib/src/http/stream.rs`'s
  test module.
