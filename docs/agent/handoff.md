# Handoff

## State

**Goal `event-streams` — door one is wired end to end except its far side.** A request that fills
the SSE cell is answered `200 text/event-stream` in place of whatever it wrote for itself
(`crates/nvs-server/src/serve.rs:@event_stream`), and the connection isolate is handed that
response's writing half and marked as writing an event stream
(`nvs_host::Isolate::over_event_stream`). `Core\Sse::current()` therefore answers inside door one,
and what the isolate sends reaches the peer as this response's body. The stage-4 `nvs-server` check
is green.

**The head an event stream answers with has one home**, `nvs_runtime::sse::MEDIA_TYPE` and
`DECLARED_HEADERS`: door two declares the list on its context and door one writes the same names
into a header map, so the two doors cannot answer with different heads.

**`Output::Capture` stays right for both doors.** A connection's `echo` is no part of either
response — § 1's bytes are frames on the socket, and § 5's events go through the writing half, so a
stray `echo` cannot land inside the framing a client is parsing.

**Door one's far side is what is left of stage 4.** `Core\Sse->receive()` and `Core\Sse\Message` are
unregistered, so a connection isolate can write events and cannot yet wait for one; three of the
four names in the stage-4 `nvs-stdlib` check are open work rather than regressions
(`current_outside_an_event_stream_is_refused_by_name` is landed).

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction, and owes the
framing's crate as well.

## Next group

**Stage 4: door one's far side, the wait that is over topics** — one file set:
`crates/nvs-stdlib/src/sse.rs`, `crates/nvs-stdlib/src/socket.rs` and
`crates/nvs-stdlib/src/topic.rs`.

- [ ] **`Core\Sse\Message` is the class the wait answers with** — the five edits at
      `crates/nvs-stdlib/src/sse.rs:125`'s roster, modelled on `Core\Socket\Message` at
      `crates/nvs-stdlib/src/socket.rs:437` and its four member bodies at
      `crates/nvs-stdlib/src/socket.rs:1090`. Only the delivery members carry over: an event stream
      is handed no peer, so there is no text or binary frame behind one and `topic()`/`value()` are
      the whole of it. `rule:concurrency/a-connection-is-a-loop`.
- [ ] **`Core\Sse->receive()` waits over topics and not over a peer** — the sibling body at
      `crates/nvs-stdlib/src/socket.rs:984`, with the peer half removed and the cross-core drain at
      `crates/nvs-stdlib/src/topic.rs:540` kept, refused by name in a streaming response per the
      goal's § *Standing decisions*. Tests
      `a_published_value_reaches_a_subscribed_event_stream_as_a_message` and
      `receive_in_a_streaming_response_is_refused_by_name`.
- [ ] **An overflowing subscriber queue answers `null` and closes that stream** —
      `rule:concurrency/connection-bounds-are-finite`'s defined close, at the queue walk
      `crates/nvs-stdlib/src/topic.rs:540` reads, as
      `an_overflowing_subscriber_queue_answers_null_and_closes_that_stream`.

## Backlog

- `docs/decisions/0176.md` — the stage 0 correction and the framing's crate, both still owed.
- Stage 5's diagnostics, phase-gated — not started; the goal's stage 5 prose is the list.
- A keepalive on an idle event stream: `nvs_runtime::sse::KEEPALIVE` has no writer yet, and its own
  doc says the connection is the half that must write it.
