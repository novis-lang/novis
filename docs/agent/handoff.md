# Handoff

## State

**Goal `event-streams` — stage 4 is landed, and door one waits.** A context now records *which*
door opened the event stream it writes (`nvs_runtime::EventStreamDoor`,
`crates/nvs-runtime/src/ctx/output.rs`), the hand-over marks `Connection`, and `Core\Topic`
admits that mark beside a peer — so a stream's isolate subscribes like any connection.
`Core\Sse->receive(): ?Core\Sse\Message` is registered and waits over topics alone: the queue
carries its own wake (`nvs_runtime::Inbox::wake_on`), so a publish on this core wakes the wait
where it stands, and an overflowed queue answers `null` and ends the body. All four stage-4
`nvs-stdlib` tests pass; the stage's other two checks were already green.

**What the wait spends, per `rule:programs/memory-priority`:** one wake pointer per connection
that ever subscribed, and one wakeup per idle stream per `CROSS_CORE_TICK`
(`crates/nvs-stdlib/src/sse.rs`, 50 ms). The tick is there because the cross-core half is still
**pull-based** — `deliver_from_other_cores` is called from inside the wait — and it goes away the
day the bus wakes the core it hands a value to. Nothing schedules that.

**`Core\Socket::receive` is still inside `Ctx::deliver`'s known gap**: it parks on its descriptor
and registers nothing on the inbox, so a delivery queued while it waits is answered by the next
call. The seam it would need now exists.

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction, and owes the
framing's crate as well.

## Next group

**Stage 5: the bounds, and the trap under them** — one file set: `crates/nvs-server/src/bounds.rs`,
`crates/nvs-server/src/serve.rs` and `crates/nvs-config/src/server.rs`.

- [ ] **The heartbeat is derived from the write wait and stays under it** — a stream that has sent
      nothing still has to move a byte before `write_idle`
      (`crates/nvs-config/src/server.rs:84`) closes it, so the heartbeat is half that wait and never
      below one second, armed where the head is answered at `crates/nvs-server/src/serve.rs:1508`.
      `rule:concurrency/connection-bounds-are-finite`.
- [ ] **The bound an event stream does not read, and the two it does** — `idle` is not armed for a
      stream (`crates/nvs-server/src/bounds.rs:81`), `lifetime` closes it however busy it was
      (`crates/nvs-server/src/bounds.rs:83`), and an event past `message`
      (`crates/nvs-server/src/bounds.rs:78`) is refused.
      `rule:concurrency/connection-bounds-are-finite`.
- [ ] **Every bound is finite with nothing configured, and the reconnect hint is drawn per stream**
      — `Connection::default` at `crates/nvs-server/src/bounds.rs:99`, a `retry:` line written at
      the open in `crates/nvs-server/src/serve.rs:1508` and not the same for two streams, and a
      drain that ends the body cleanly rather than resetting.
      `rule:concurrency/connection-bounds-are-finite`.

## Backlog

- Stage 6's rulebook and record: `docs/decisions/0176.md` is open and owes the framing's crate.
- `Core\Socket::receive` could park over both sources now — `nvs_runtime::Ctx::deliver`'s known gap.
- A pushing cross-core bus removes `CROSS_CORE_TICK` — that const's own doc in
  `crates/nvs-stdlib/src/sse.rs` is where the trade is written down.
- `Core\Sse->receive` on a stream that subscribed to nothing parks until its lifetime bound; stage 5
  is where that becomes observable.
