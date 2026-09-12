# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled connections.
Stages 1–5 are on disk, and stage 6's surface is: `Core\Http\Stream` and its four readers exist and
work. Nothing of stages 7–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

`Core\Http\Client` carries an eighth row, `stream` (`crates/nvs-stdlib/src/http.rs:891`), taking a
`Core\Http\Method` ahead of the URL as `request` does and answering `Core\Http\Stream`
(`crates/nvs-stdlib/src/http/stream.rs:158`). Both rows share one body: `exchanged`
(`crates/nvs-stdlib/src/http.rs:1969`) is the whole call, and which class the answer is built into is
the only difference between a buffered reply and a streamed one. `header`/`headers` are likewise one
implementation behind two doors (`joined_field`, `crates/nvs-stdlib/src/http.rs:2065`).

**The body is taken, not shared**: `consumed` (`crates/nvs-stdlib/src/http/stream.rs:488`) moves the
octets out of the stream's slot, so `events()`, `lines()`, `chunks()` and `saveTo()` are four
framings of one walk and the second one throws naming the first —
`rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`. `events()`
parses the WHATWG EventSource format whole, including the `id` the format keeps in force between
events; `saveTo` delegates to `crate::io::stream_to_disk` with a required `max`.

**What is not yet true of it: the body does not arrive lazily.** `exchanged` hands the whole reply
over with the head, so a walk frames octets already in hand, `[http.client]` has no `idle` or
`max_duration` and `Core\Http\Options` has no key for either. That is the next group, and it is the
one thing standing between this class and the rule it is written against. `tls()`, which
`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime` also names on the head, waits
for § 13's TLS session and is not in stage 6.

Nothing is blocked and no design question is open.

## Next group

**Stage 6: the streamed body runs under its own two bounds** — one file set:
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-stdlib/src/http/stream.rs`, `crates/nvs-config/src/tree.rs`.

- [ ] **`idle` and `maxDuration` are a stream-only bag, and `[http.client]` ships both** — the two
      keys cannot join `OPTIONS` (`crates/nvs-stdlib/src/http.rs:379`), which every buffered row
      shares and on which `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime` makes
      either key a diagnostic. `request_params!` (`crates/nvs-stdlib/src/http.rs:1127`) is the shape
      to copy: turn `OPTIONS` into the same kind of macro, take a trailing group, and spell
      `STREAM_OPTIONS` with it, then widen the `stream` row's arity from `REQUEST_ARITY + 1`
      (`crates/nvs-stdlib/src/http.rs:786`). The directives are `Runtime` per the goal's § *Standing
      decisions*; `crates/nvs-config/src/tree.rs` and `crates/nvs-config/tests/tree.rs` are the other
      half. This closes `tests/conformance/reject/http-client-idle-on-a-buffered-member-is-refused.nvst`.
- [ ] **The transport hands over a reader rather than a body** — `send`
      (`crates/nvs-stdlib/src/http/transport.rs:288`) reads the whole reply through `exchange`
      (`:404`) and `parse` (`:800`); a stream needs the head parsed and the connection kept, which
      is where the two bounds above are applied and where `Core\Db\Stream`'s shape
      (`crates/nvs-stdlib/src/db/stream.rs:64`) is the precedent for holding live state off the
      object. `crates/nvs-stdlib/src/http/stream.rs:546`'s `step` is the one reader that changes:
      its `body` slot becomes the handle. Retry stays before the head only
      (`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`).
- [ ] **The three `-p nvs-stdlib` cases stage 6's second check names** —
      `stream_idle_ends_a_silent_reply`, `stream_max_duration_ends_an_endless_reply` and
      `stream_is_retried_before_its_head_and_never_after`, beside the module's existing listener
      tests at `crates/nvs-stdlib/src/http.rs:2543`.

## Backlog

- `Core\Http\Stream::tls()` — the head reader ADR 0180 § 13 owns, waiting for the TLS session.
- Decoding a streamed reply's content coding (§ 8 names gzip, brotli and zstd for a stream too).
- `Core\Test::answerHttp` cannot answer a stream in pieces, so no case can pin a chunk boundary.
- The goal's `[context] modules` did not name `crates/nvs-types/src/core_lib.rs`, whose closed
  tainted-answer roster fails on any new qualified member.
