# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–5 are on disk. Stage 6's surface is complete and its transport half now is
too: a streamed call stops at the head, and the body that follows is bounded by `idle` and
`maxDuration`. What is not yet true is the *shape* of that body — it is still drained whole into
`Core\Http\Stream`'s octet slot instead of being walked. Nothing of stages 7–14 is.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

`transport::Incoming` (`crates/nvs-stdlib/src/http/transport.rs:331`) is the reply body as a reader:
`exchange` stops at the blank line, hands it the connection and whatever octets came with the head,
and it frames chunked, sized and until-close bodies one read at a time under two bounds it is built
with. `send` is that reader drained in one go; `send_streamed`
(`crates/nvs-stdlib/src/http/transport.rs:648`) is the same call under `idle`/`maxDuration`, which
`exchanged`'s `streamed` arm (`crates/nvs-stdlib/src/http.rs:2065`) is the only caller of. Both
`[http.client]` keys are read now and the `[unread:]` notes are off.

**What is not yet true: the body is a `Vec` by the time a program sees it.** `exchanged` calls
`Incoming::whole` for the streamed member too, so `Core\Http\Stream`'s body slot still holds octets
and the walk still indexes them. Both stage 6 checks are green regardless — the three transport
tests assert the bounds where they are enforced.

Nothing is blocked and no design question is open.

## Next group

**Stage 6: the walk runs on the reader rather than on a `Vec`** — one file set:
`crates/nvs-stdlib/src/http/stream.rs`, `crates/nvs-stdlib/src/http.rs`,
`crates/nvs-runtime/src/ctx/held.rs`.

- [ ] **The stream's body slot holds a handle, not octets** — `Incoming` is a Rust value and a slot
      holds a `Value`, so it goes in a request-owned table and the slot carries the key, which is
      `Core\IO\File`'s design (`crates/nvs-runtime/src/ctx/held.rs:222` and its `HeldSocket`
      trait). `exchanged` (`crates/nvs-stdlib/src/http.rs:2065`) stops calling `whole` on the
      streamed arm and files the reader instead; `consumed`
      (`crates/nvs-stdlib/src/http/stream.rs:488`) moves the key rather than the octets, which is
      what keeps "read once" a property of the memory.
      `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`.
- [ ] **The three walks frame off the reader** — `step`
      (`crates/nvs-stdlib/src/http/stream.rs:546`) reads its octets out of a slot and indexes them
      by `READER_FROM_AT`; against a socket it becomes: frame from `Incoming::held()`, and where
      that is not a whole element yet, `pull()` and try again, with `false` the end of the body.
      `line_at` and `event_at` already take a `&[u8]` and an offset, so the caps stay where they
      are. `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
- [ ] **`saveTo` and the answer table walk the same reader** — `saveTo`
      (`crates/nvs-stdlib/src/http/stream.rs:925`) is `Core\IO::writeStream` over the same walk
      (`rule:core-classes/io-write-stream`), and `faked` (`crates/nvs-stdlib/src/http.rs:2313`) has
      to hand out a reader over a `Vec` so a test's stream and a real one are one implementation —
      an `Incoming` with no source behind it is that.

## Backlog
- Stage 7's pool wants `Incoming` to say whether it framed to the end, so a connection only goes
  back after a fully framed reply — `docs/agent/loop-goal.toml` stage 7.
- A `HEAD` reply with a `Content-Length` and no body waits for octets that never come; no member
  sends one today — `crates/nvs-stdlib/src/http/transport.rs`.
- Content codings are not decoded anywhere yet; stage 7 owns gzip, brotli and zstd.
