# Handoff

## State

**Goal 6, M7 — ADR 0083 § 4's first two rows are on disk.** `Core\Topic::subscribe` and
`unsubscribe` live in `crates/nvs-stdlib/src/topic.rs` over a per-core `thread_local!` table from a
topic's name to the subscribers on this core. The topic name is a `CoreTy::Text(Qual::Sink)`, so
§ 4's refusal of a `tainted` name is the signature's and never a body's, and both members check the
name *before* they ask whether this context is a connection's — that module's own docs are the home
of why, and of what the table spends.

**The delivery queue moved off the `Ctx` and behind an `Rc`.** `nvs_runtime::Inbox`
(`crates/nvs-runtime/src/peer.rs:131`) is § 3's second source as a thing two owners can hold: the
connection's context holds the only strong reference and the subscriber table holds a `Weak`, so a
connection that ended at a limit or a fatal error unsubscribes itself by being dropped and the table
stays O(live connections). `Ctx::inbox` (`crates/nvs-runtime/src/ctx/isolate.rs:108`) is the handle,
made on the first subscribe rather than per request, so an ordinary request pays no allocation for
it.

**`publish` — § 4's second row — is unwritten, and it is the whole of what is left.** The fan-out,
ADR 0023's copy of the published value, the cross-core hand-off and § 4's bounded queue are all
inside it; the table has no filler but a `#[test]` until it lands.

**The goal's failing acceptance check is still open**: `nvs-server`'s
`a_publish_reaches_a_subscriber_on_another_core` is item 2 below, and item 1 has to land first.

## Next group

**ADR 0083 § 4's `publish`, in the three questions it is.** All three share
`crates/nvs-stdlib/src/topic.rs` and `crates/nvs-runtime/src/peer.rs`; the first has to land before
either of the others, and the second is what closes the goal's failing check.

- [ ] **`Core\Topic::publish`, the graph copy and the fan-out on this core** — ADR 0083 § 4's second
      row. A third row beside the two in `crates/nvs-stdlib/src/topic.rs:91`, its walk shaped like
      the pruning one in `crates/nvs-stdlib/src/topic.rs:235`; the value crosses by
      `crates/nvs-runtime/src/graph.rs:515`'s `copy_graph`, once per subscriber because § 4 says
      subscribers share nothing with each other or with the publisher, and each copy is queued with
      `crates/nvs-runtime/src/peer.rs:131`'s `Inbox::push`. It answers the count delivered to. A
      `secret` may not be published (ADR 0033), which `copy_graph` already refuses.
- [ ] **The hand-off to another core** — ADR 0083 § 4's "a publish from a connection on core 3
      reaches subscribers on core 0", which is the goal's failing check. An `Rc<Inbox>` is not
      `Send`, so what crosses is a *message* and not a handle:
      `crates/nvs-server/src/serve.rs:1239`'s `serve_on_this_core` is one core's loop and the place
      a per-core mailbox has to be registered and drained. Decide and record the shape — a boot-time
      registry of per-core senders is the obvious one — in `crates/nvs-stdlib/src/topic.rs`'s module
      doc, under the standing decision that a design call is made and written down rather than
      raised.
- [ ] **The bounded queue, and closing a slow subscriber** — ADR 0083 § 4's priority-1 rule, at
      `crates/nvs-runtime/src/peer.rs:131`. `Inbox` has the `len` the bound reads; past it *that
      subscriber's connection* is closed with a defined code and a metric increments — the publisher
      is never blocked. `crates/nvs-runtime/src/ctx/isolate.rs:108` is the other half, because
      closing reaches the peer the context holds.

## Backlog

- The wake seam: a delivery queued while the isolate is parked in the socket read is answered by the
  *next* `receive()` — `crates/nvs-runtime/src/ctx/isolate.rs`'s `Ctx::deliver` known gap owns it,
  and it becomes observable the moment `publish` lands.
- § 1's `[limits] idle` and § 3's send timeout — `crates/nvs-server/src/socket.rs`'s § *What is not
  here yet*.
- `docs/novis.md` is **generated** from the registry cards by `verify.py`'s `reference` step, so a
  new row leaves it dirty after the wrap has committed — stage it with the slice rather than writing
  it by hand.
