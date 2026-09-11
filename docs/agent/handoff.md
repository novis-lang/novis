# Handoff

## State

**Goal `event-streams` — stage 5 has opened, and the keep-alive is the first bound landed.** An event
stream that sends nothing now writes `nvs_runtime::sse::KEEPALIVE` at
`crate::bounds::heartbeat(write_idle)` — half the response wait, floored at a second wherever the wait
leaves room, and strictly under it at every duration `nvs_config::server::waits_for` accepts. That
last property is load-bearing rather than tidy: the beat is filed as the connection task's *one*
deadline (`nvs_host::Timers`), so it is sound only because it always lands earlier than the socket's.

`crate::serve::Answer::Streaming` is now a struct variant carrying `Option<Heartbeat>`, `None` for a
request-scoped stream — a streamed response is bounded by the request writing it, and quiet inside one
is a fault where quiet on an event stream is the ordinary state. `event_stream`
(`crates/nvs-server/src/serve.rs:1598`) is the one caller that arms it. The body publishes the next
beat into `crate::bounds::NextBeat`; the drive loop files it, and the playbook bullet above is why.

**The rustdoc gate is green again** — `crates/nvs-runtime/src/peer.rs:226`'s `Waker` link was
unqualified. `python tools/verify.py --doc` passes clean.

**`Core\Socket::receive` is still inside `Ctx::deliver`'s known gap**, and record
`docs/decisions/0176.md` is still open with the stage 0 correction and owes the framing's crate.

## Next group

**Stage 5: the bounds, and the trap under them** — one file set: `crates/nvs-server/src/bounds.rs`,
`crates/nvs-server/src/serve.rs` and `crates/nvs-runtime/src/sse.rs`.

- [ ] **The bound an event stream does not read, and the two it does** — `Connection::idle`
      (`crates/nvs-server/src/bounds.rs:78`) closes a connection whose *peer* stopped speaking, and an
      event stream's peer never speaks, so arming it would close every healthy stream; `lifetime` is
      armed and closes one however busy it was, and `message` bounds one event rather than one frame.
      Armed beside the heartbeat at `crates/nvs-server/src/serve.rs:1598`, refused where the event is
      framed at `crates/nvs-runtime/src/sse.rs:112`.
      `rule:concurrency/connection-bounds-are-finite`.
- [ ] **Every bound is finite with nothing configured, and the reconnect hint is drawn per stream** —
      `Connection::default`'s destructuring case (`crates/nvs-server/src/bounds.rs:78`) is the first
      half and already stands; the second is `nvs_runtime::sse::reconnect_after`
      (`crates/nvs-runtime/src/sse.rs:112`) written once at open with a jittered wait, so two streams
      reconnecting do not return together, and a drain ends the body rather than resetting the
      connection. `rule:concurrency/connection-bounds-are-finite`, `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`.

## Backlog

- Door two gets no heartbeat: a `text/event-stream` opened through `Core\Response::stream` is a
  streamed response, and `Answer::beating` is only reached from `event_stream`. Decide in stage 6.
- Stage 6's rulebook and record: `docs/decisions/0176.md` is open and owes the framing's crate.
- `Core\Socket::receive` could park over both sources now — `nvs_runtime::Ctx::deliver`'s known gap.
- A pushing cross-core bus removes `CROSS_CORE_TICK` — that const's own doc in
  `crates/nvs-stdlib/src/sse.rs` is where the trade is written down.
- `Core\Sse->receive` on a stream that subscribed to nothing parks until its lifetime bound; the item
  above is where that becomes observable.
