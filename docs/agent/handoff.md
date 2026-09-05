# Handoff

## State

**Goal 6, M7 — ADR 0083 § 4 is whole.** The bus crosses cores (`crates/nvs-stdlib/src/bus.rs`) and
the fan-out now keeps § 4's priority-1 bound: each subscriber's queue holds
`nvs_runtime::INBOX_CAP` (256) deliveries, `Core\Topic::publish` asks each queue for room *before*
it makes that subscriber's copy, and a full one is marked and stepped over — so a publisher is
never blocked, never allocates for a subscriber it is skipping, and does not count it. **Who closes
was the design question and the answer is the subscriber**: the peer is a field of the subscriber's
own `Ctx`, possibly on another core, so the overflow is raised on the `Inbox` both sides already
hold and `Core\Socket::receive()` obeys it — it closes the peer with `Closing::SlowSubscriber`
(RFC 6455's 1008) and answers § 3's `null`. `nvs_runtime::peer`'s module doc is the one home of
that reasoning, of what the queue spends, and of the metric.

`PeerSocket::close` now takes a `Closing`, which is this tree's only place a close code is decided.
`nvs_runtime::slow_subscribers_closed()` is § 4's "a metric increments", per core — **known gap:
nothing exports it**, because it is not one of ADR 0076 § 1's series and the registry that would
carry it is `nvs_server::metrics`. That is item 3 below.

**The rustdoc gate is green again** — `peer.rs`'s module doc linked `[`Ctx`]` from a module that
does not import it; the playbook bullet owns the general shape.

**The remaining § 4/§ 3 gap is the wake seam**, unchanged and now the only one: a delivery queued
while its connection is parked inside `PeerSocket::receive` is answered by the *next* `receive()`.
`nvs_runtime::Ctx::deliver`'s known gap states it.

## Next group

**The connection's two remaining waits, and the metric that watches them.** Items 1 and 2 share
`crates/nvs-server/src/socket.rs` and `crates/nvs-stdlib/src/socket.rs`; item 3 is small and shares
the first of those. Take 1 first — it is the hard one and it decides the shape the other two sit in.

- [ ] **The wake seam: a park over both sources** — ADR 0083 § 3's "`receive()` suspends until the
      next thing arrives from *either* side". Today the park is on the socket alone, so a topic
      delivery queued while the connection is already parked waits for a peer frame that may never
      come. The gap is stated at `crates/nvs-runtime/src/ctx/isolate.rs:72` and the two ends that
      have to take part are `crates/nvs-stdlib/src/socket.rs:1013`'s drain-then-read ordering and
      `crates/nvs-server/src/socket.rs:158`'s `Framed::receive`, which is where the codec blocks on
      `nvs_host::NvsTcp`. **The design question to settle first is what wakes the park**: the
      publisher is on another task and may be on another core, so the likely shape is a wakeable
      handle the `Inbox` carries and the publisher signals, with the framing layer's read taking
      part in a select rather than owning the wait — `crates/nvs-host/src/net.rs`'s parking stream
      is the half that knows how a task is resumed. Landing it also closes the overflow's one
      remaining latency: a subscriber being closed only learns so at its next wait.
- [ ] **A connection's `[limits] idle`, and § 3's send timeout** — both are named as not landed in
      `crates/nvs-server/src/socket.rs:45`'s *What is not here yet*, and both are ADR 0074's rule
      that no wait has an unbounded spelling. A peer that opens a connection and goes silent holds
      one isolate for ever; `crates/nvs-server/src/socket.rs:180`'s `send` doc records the other
      half. The deadline belongs on the stream `crate::io::ConnectionIo::into_stream` hands over.
- [ ] **§ 4's metric becomes an ADR 0076 series** — `crates/nvs-runtime/src/peer.rs:169`'s count read
      into the registry at `crates/nvs-server/src/metrics.rs:148`, where § 1's nine series are
      declared. It is a wiring change, not instrumentation: the count is already maintained. Decide
      the name with ADR 0076 § 1's table in hand (`nvs_topic_subscribers_closed_total` is the shape
      the neighbours take) and say in that ADR's § 1 that it is one more row.

## Backlog

- A `.nvst` case cannot reach § 4's overflow: a case runs a script handed no socket, which is why
  the bound is pinned by `#[test]`s — `crates/nvs-stdlib/src/socket.rs:1013`'s own comment.
- `INBOX_CAP` is a constant, not a directive; a `[limits]`-shaped spelling is ADR 0005's question if
  a deployment ever needs one — `crates/nvs-runtime/src/peer.rs:154` owns why 256.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`.
