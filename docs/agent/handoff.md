# Handoff

## State

**Goal 6, M7 — ADR 0083 § 4's qualifiers at the connection boundary are whole, and the acceptance
check that named them is green.** § 3's received payload was already `tainted`
(`nvs_stdlib::socket`'s `Message::text` answers `?tainted string`) and § 4's topic name was already
a `Qual::Sink` on all three `Core\Topic` rows; what was missing was the third qualifier rule —
**a `secret` may never be published**. `nvs_types::expr::quals`'s `reject_secret_published_argument`
is that half, reporting `E0775` at the written argument through the same `reject_secret_crossing`
`Core\Serialize::encode`, `spawn script`'s `args:` and `Core\Socket::upgrade`'s `args:` report it
through — one code for one graph copy, which is that code's own doc. The value is found by its
`ArgSlot`, so `value:` is refused as surely as the second positional argument; the topic is
deliberately not asked about, because a qualified name is already a sink mismatch there.

**§ 4 is otherwise unchanged from the last session.** The bus crosses cores, each subscriber's queue
holds `nvs_runtime::INBOX_CAP` (256), and a subscriber that overflows closes itself with
`Closing::SlowSubscriber`. `nvs_runtime::peer`'s module doc is the one home of that reasoning, of
what the queue spends, and of the metric. `nvs_runtime::slow_subscribers_closed()` is § 4's "a
metric increments", per core — **known gap: nothing exports it**, because the registry that would
carry it is `nvs_server::metrics`. That is item 3 below.

**The remaining § 4/§ 3 gap is the wake seam**, unchanged and still the only one: a delivery queued
while its connection is parked inside `PeerSocket::receive` is answered by the *next* `receive()`.
`nvs_runtime::Ctx::deliver`'s known gap states it, and item 1 is it.

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
- [ ] **A connection's `[limits] idle`, and § 3's send timeout** — ADR 0083 § 2's `limits: {idle:
      5m}` and § 3's "`send()` … throws on the send timeout rather than waiting forever"
      (ADR 0074). Neither is enforced: `crates/nvs-server/src/socket.rs:158` parks with no deadline
      and `crates/nvs-stdlib/src/socket.rs:1013`'s siblings buffer with none either. `[server]`'s
      four waits are already durations in `nvs_config::server`, so what is missing is the deadline
      travelling with the peer rather than a new directive.
- [ ] **§ 4's metric becomes an ADR 0076 series** — `crates/nvs-runtime/src/peer.rs:169`'s per-core
      count is read by nothing. ADR 0076 § 1's table is the nine series a core meters, and
      `crates/nvs-server/src/metrics.rs:1` is the registry that would carry a tenth; § 7's
      `max_series` bound applies unchanged.

## Backlog

- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`; a decided-and-recorded call in `Core\Request`'s module doc if it is needed.
- `Core\Validate` has no text member, so ADR 0083 § 3's named launderer is spelled as ADR 0024
  § 2's checked conversion today — `crates/nvs-stdlib/src/validate.rs`, and `tests/sockets.rs`
  says so where it asserts the laundered half.
