# Handoff

## State

**Goal `event-streams` — stage 4's stdlib half is landed.** `Core\Sse::current()` answers the event
stream a program is already writing and `Core\Sse->retry()` puts the reconnection time on the wire as
a block of its own; the classification each `send` parameter carries is pinned in `nvs-types`, so
that stage-4 check is green.

**The question `current()` asks is now a fact the context records** —
`crates/nvs-runtime/src/ctx/output.rs:543`'s `mark_event_stream` / `has_event_stream`. It is neither
of the two obvious ones: an event stream is handed no peer, so the sibling class's question answers
`false` inside the very isolate this must answer for, and an ordinary streaming response holds a
writing half, so *is a body open* answers `true` for a program this must refuse.

**Door one's far side is still unwired.** `Core\Sse->receive()` and `Core\Sse\Message` are
unregistered, and `serve.rs` still answers a request that filled the SSE cell with the handler's own
response — so the stage-4 `nvs-server` check and two of the four names in the `nvs-stdlib` one are
open work rather than regressions.

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction, and owes the framing's
crate as well.

## Next group

**Stage 4: door one's wiring, the response the connection answers with** — one file set:
`crates/nvs-server/src/serve.rs`, `crates/nvs-host/src/isolate.rs` and
`crates/nvs-runtime/src/ctx/output.rs`.

- [ ] **The connection isolate is handed a body and marked as writing an event stream** — the
      `over_socket` sibling at `crates/nvs-host/src/isolate.rs:331`, reached from
      `crates/nvs-host/src/isolate.rs:278`'s cell, calling `set_body_stream` and
      `crates/nvs-runtime/src/ctx/output.rs:543`'s `mark_event_stream` at the point the socket is
      written at today. Until this lands `Core\Sse::current()` refuses inside door one, which is the
      one thing this member cannot answer for itself. `rule:concurrency/a-connection-is-a-root-isolate`.
- [ ] **`serve.rs` answers `200 text/event-stream` in place of the handler's own response** — the
      arm at `crates/nvs-server/src/serve.rs:1125`, where the cell is already taken and the isolate
      already started, modelled on the head-early path at `crates/nvs-server/src/serve.rs:1026`.
      `rule:concurrency/two-doors-one-isolate`, and goal prose stage 4 item 18.
- [ ] **The three `nvs-server` tests the stage names**, beside the stage-3 ones at
      `crates/nvs-server/src/serve.rs:3865`: the media type and status, the isolate's bytes reaching
      the peer as this response's body, and the upgrading request's arena released before the
      stream's isolate starts.

## Backlog

- `Core\Sse->receive()` and `Core\Sse\Message`, topics only — goal prose stage 4 item 16.
- The two topic tests in the stage-4 `nvs-stdlib` check — a published value reaching a subscribed
  stream, and an overflowing queue closing it.
- Stage 5's bounds over an event stream — `crates/nvs-server/src/bounds.rs`, goal prose stage 5.
- `docs/decisions/0176.md` still owes the stage 0 correction and the framing's crate.
