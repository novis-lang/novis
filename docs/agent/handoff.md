# Handoff

## State

**Goal 6, M7 — ADR 0083 § 3's connection surface is on disk.** `Core\Socket::current()` answers a
handle inside the isolate an upgrade opened, `receive()` is the one wait over both sources,
`send()`/`sendBytes()` are RFC 6455's two payload kinds, and `Core\Socket\Message`'s four readers
(`topic`, `text`, `bytes`, `value`) are what each kind carries. `crates/nvs-stdlib/src/socket.rs`
owns all of it; the class doc on `MESSAGE` is the home of why there is one message class and not
two.

**The second source is a queue on the context, not on the socket.** `nvs_runtime::Delivery` plus
`Ctx::deliver`/`Ctx::take_delivery` sit beside `Ctx::peer`, so `receive()` drains the bus first and
only then parks on the peer — which is what makes it the *one* wait § 3 specifies without the
framing layer learning that topics exist. `Ctx::deliver`'s doc comment is the home of the ordering
and of its known gap: a delivery queued while the isolate is already parked in the socket read is
answered by the *next* `receive()`, because the park is on the socket alone. Nothing can observe
that yet — § 4's bus is unwritten, so the only publisher is a test on the same task — and the wake
seam belongs to the slice that writes the publisher.

**`Core\Socket` is the first `Core` class with instance members and no slots**, because a
connection's whole state is its isolate's. Two things moved for it and both name it: the skip in
`nvs_stdlib::instance::descriptors`, and the `CONTEXTUAL` list in
`registry::tests::a_class_with_slots_has_instance_members_and_the_reverse`.

**The goal's failing acceptance check is closed** — all four of its `-p nvs-stdlib` tests run.

## Next group

**ADR 0083 § 4's `Core\Topic`, which is the only thing that can fill `receive()`'s second source.**
All three share `crates/nvs-stdlib/src/topic.rs` (new), `crates/nvs-runtime/src/ctx/isolate.rs` and
`crates/nvs-stdlib/src/registry.rs`; the first has to land before either of the others.

- [ ] **`Core\Topic::subscribe`/`unsubscribe` and the per-core subscriber table** — ADR 0083 § 4's
      first two rows. The class goes in a new `crates/nvs-stdlib/src/topic.rs` registered at
      `crates/nvs-stdlib/src/registry.rs:1474`, beside `crate::socket::MESSAGE`; the topic name
      refuses `tainted`, which is `CoreTy::Text(Qual::Sink)` for § 4's stated reason.
- [ ] **`Core\Topic::publish`, the graph copy and the fan-out** — ADR 0083 § 4. It answers the
      count delivered, copies with `nvs_runtime::copy_graph` as
      `crates/nvs-stdlib/src/socket.rs:1010`'s upgrade already does, and hands each subscriber a
      `nvs_runtime::Delivery` through `crates/nvs-runtime/src/ctx/isolate.rs:76`.
- [ ] **The bounded queue, and closing a slow subscriber** — ADR 0083 § 4's priority-1 rule. The
      queue is `crates/nvs-runtime/src/ctx/mod.rs:988`'s `deliveries`; overflow closes *that*
      connection and never blocks the publisher.

## Backlog

- The wake seam: a delivery published while a connection is parked in the socket read — owned by
  `Ctx::deliver`'s known gap at `crates/nvs-runtime/src/ctx/isolate.rs:76`.
- § 4's cross-core hand-off. This session's queue is per-context and per-core only.
- `[context] adrs` in `docs/agent/loop-goal.toml` has no `0083 §4`; the next session pays for it.
- ADR 0083 § 1's `[limits] idle` and the send timeout's own number — `nvs_server::socket`'s
  *What is not here yet*.
