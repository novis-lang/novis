# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–6 are on disk and stage 6 is now complete in shape as well as in bounds: a
streamed reply is a reader from the head onwards, and the four framings walk it one element at a
time. Nothing of stages 7–14 is.** [ADR 0180](../decisions/0180.md) is the record and the home of
every decision this goal executes.

A streamed call files `transport::Incoming` against the request
(`crates/nvs-runtime/src/ctx/held.rs:292`, `HeldReader` and its table) and `Core\Http\Stream`'s body
slot holds the key; `consumed` moves that key to the one walk that takes the body, and `step`
(`crates/nvs-stdlib/src/http/stream.rs:565`) frames an element off what has arrived, pulls when it
cannot, and takes the reader out of the table at the end of the body — which is what gives the
connection back. An armed answer table files its canned octets the same way
(`transport::Incoming::already`), so a test walks the code a call to an origin walks.

Nothing is blocked and no design question is open.

## Next group

**Stage 7: a connection is reused, and a reply may arrive compressed** — one file set:
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/compress.rs`,
`crates/nvs-stdlib/src/http.rs`.

- [ ] **A connection outlives its call, in a per-core pool keyed by the pinned address** — `one`
      opens one and drops it per attempt (`crates/nvs-stdlib/src/http/transport.rs:769`), and the
      head still says `Connection: close` (`crates/nvs-stdlib/src/http/transport.rs:906`). The shape
      is `crates/nvs-runtime/src/pool.rs:1`'s: a `thread_local!` `Vec` the calling core owns, keyed
      by what the caller hands over rather than by anything the pool parses.
      `rule:security/db-pool-reset-is-a-boundary` for the per-core bound,
      `rule:http-server/allow-url-pins-the-address` for why the key is the pinned address.
- [ ] **A connection goes back only after a fully framed reply** — `Incoming` knows when the body
      ended (`crates/nvs-stdlib/src/http/transport.rs:461`, `pull` and the `ended` flag beside it)
      and `step` is where the reader is dropped today
      (`crates/nvs-stdlib/src/http/stream.rs:565`); a body that stopped short, or one the walk
      abandoned, closes rather than returns.
      `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`.
- [ ] **The request offers gzip, brotli and zstd and the reply is decoded under the compress
      bound** — the head writes `Accept: */*` and no `Accept-Encoding`
      (`crates/nvs-stdlib/src/http/transport.rs:906`), and `decompress_within`
      (`crates/nvs-stdlib/src/compress.rs:522`) is the bound both halves share. A coding the
      request did not offer is refused naming it. `rule:core-classes/decompression-bound`.

## Backlog

- Stage 7's client identity is a second check (`docs/agent/loop-goal.toml:9485`) and its own group:
  a PEM chain over a key pair, part of the pool key.
- A dribbling origin is only reachable from `transport.rs`'s own tests; the walk over a reader is
  pinned at the framing functions rather than end to end — `crates/nvs-stdlib/src/http/stream.rs`'s
  test module.
