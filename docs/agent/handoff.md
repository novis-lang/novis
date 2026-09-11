# Handoff

## State

**Goal `event-streams` — door one is wired end to end except the wait on its far side.** A request
that fills the SSE cell is answered `200 text/event-stream`, the connection isolate is handed that
response's writing half and marked as writing an event stream, and `Core\Sse::current()` and
`send()` both answer inside it. The stage-4 `nvs-server` and `nvs-types` checks are green.

**`Core\Sse\Message` is registered** (`crates/nvs-stdlib/src/sse.rs:@MESSAGE`) with two readers and
not four: `topic(): string` — never `null`, because an event stream is handed no peer and so has no
second kind of message — and `value(): mixed`. Nothing builds one yet, so its three conformance
cases are all refusals, and `crate::instance::read_slot` is now the one home of the receiver check,
the slot borrow and the retain that both message classes' readers share.

**Two facts the wait rests on, both found this session and neither landed:**

1. **`Core\Topic::subscribe` refuses inside an event-stream isolate today.** `connection_inbox` asks
   `has_peer()` (`crates/nvs-stdlib/src/topic.rs:383`), which is `false` on door one. The two doors
   are *indistinguishable* on the context as it stands — a request streaming an ordinary body and a
   connection isolate both carry a `body_stream` and both have `event_stream` marked — so the
   hand-over needs a mark of its own at `crates/nvs-host/src/isolate.rs:540`, and `subscribe`,
   `receive` and the streaming-response refusal all read that one predicate.
2. **Nothing wakes a parked subscriber, and on door one that is a hang rather than a delay.**
   `crates/nvs-stdlib/src/bus.rs` fires no wake at all and `deliver_from_other_cores` is pull-based,
   called only from inside `receive()` itself; the socket's wait gets away with it by parking on the
   *socket*, which an event stream does not have. So stage 4's item 16 — "`Reactor::remote_wake` …
   reused from the WebSocket path unchanged" — names machinery that is not there. The safe option is
   a waker on `nvs_runtime::peer::Inbox` (`crates/nvs-runtime/src/peer.rs:197`), which makes a
   same-core publish wake the wait at once, plus a bounded re-check tick for the cross-core drain;
   the tick costs one wakeup per idle stream per interval, and it is what a pushing bus would later
   remove. Say what it spends where it lands, per `rule:programs/memory-priority`, and record the
   choice with the stage-6 record.

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction, and owes the framing's
crate as well.

## Next group

**Stage 4: door one's far side, the wait that is over topics** — one file set:
`crates/nvs-stdlib/src/sse.rs`, `crates/nvs-stdlib/src/topic.rs` and `crates/nvs-runtime/src/peer.rs`.

- [ ] **An event-stream connection is a mark of its own, and both waits read it** — the hand-over at
      `crates/nvs-host/src/isolate.rs:540` marks more than `mark_event_stream`
      (`crates/nvs-runtime/src/ctx/output.rs:543`), and `connection_inbox` at
      `crates/nvs-stdlib/src/topic.rs:382` admits it beside a peer, so a connection isolate can
      subscribe. `rule:concurrency/two-doors-one-isolate`.
- [ ] **`Core\Sse->receive(): ?Core\Sse\Message` waits over topics and not over a peer** — the five
      edits at `crates/nvs-stdlib/src/sse.rs:125`'s roster, modelled on the sibling body at
      `crates/nvs-stdlib/src/socket.rs:1012` with the socket read replaced by the park State 2 above
      settles, and `message_of_delivery` at `crates/nvs-stdlib/src/socket.rs:897` is the builder to
      copy. A streaming response is refused by name.
      `rule:concurrency/a-connection-is-a-loop`.
- [ ] **An overflowing subscriber queue answers `null` and closes that stream** — the overflow is
      already raised on the inbox and read at `crates/nvs-stdlib/src/socket.rs:1018`; what differs is
      the close, which is `Emit::finish` at `crates/nvs-runtime/src/stream.rs:217` reached through
      `crates/nvs-stdlib/src/sse.rs:@onto_the_wire`'s half of the context rather than a peer's close
      code. `rule:core-classes/topic`.

## Backlog

- Three `.nvst` cases are owed for `Core\Sse::receive` by the floor at
  `crates/nvs-stdlib/tests/conformance_coverage.rs:156` — the refusal outside a connection, the loop
  shape, and the streaming-response refusal.
- `docs/decisions/0176.md` is unwritten — stage 6, and it owes the two doors, `receive()`'s
  amendment of § 5 and the framing's crate.
- The pack's `[context] modules` names neither `crates/nvs-runtime/src/peer.rs` nor
  `crates/nvs-stdlib/src/bus.rs`, and the wait's whole design rests on both; the driver's sweep will
  not add them, since no commit of this session touched either.
