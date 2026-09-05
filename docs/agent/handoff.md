# Handoff

## State

**Goal 6, M7 — ADR 0083 § 1 is landed and pinned.** The upgrade opens a root isolate from the
connection's own context, the upgrading request answers its own ordinary response, and its arena
is gone before the connection isolate is built — `serve_connection`'s doc
(`crates/nvs-server/src/serve.rs:542`) states that ordering and
`the_upgrading_requests_arena_is_released_while_the_connection_is_open` is the reading that says
it had its effect. **Still no `101`**: nothing can frame a byte until the framing slice, and that
function's doc owns the reasoning. **The member still throws** (`crates/nvs-stdlib/src/socket.rs:155`
is its row, and the body below it).

**The request path's retention is fixed.** `nvs_host::Scheduler` files a `Finished` — the whole
`Ctx` — for a **root** task only; a child's context is dropped the moment its task returns,
because a child's answer comes back through whatever spawned it and never through that list.
`nvs_host::Finished`'s doc (`crates/nvs-host/src/scheduler.rs:540`) is the home of the decision and
of what it spends. The ids every ended task owes ADR 0115 § 2 rule 3 moved to one **drained**
`Scheduler::take_ended`, which replaced both the non-draining `finished()`/`cancelled()` readers:
`run_until_idle` was re-walking every task a worker had ever served, once per turn.
`Scheduler::take_cancelled` is gone with them — `RunReport::cancelled` is the count.

**§ 5's check still needs a decision before its test can exist.** The slot is offered only where
`hyper` left an `OnUpgrade` (`crates/nvs-server/src/serve.rs:699`), and SSE is not an HTTP upgrade
— a `Core\Sse::upgrade` request is an ordinary `200` — so `sse_is_a_connection_isolate_with_no_receive`
cannot reach a slot on today's door. That is the goal's one remaining § 1/§ 5 acceptance failure.

## Next group

**The member, then the door's second way in.** Both items are `crates/nvs-stdlib/src/socket.rs`
plus the slot at `crates/nvs-runtime/src/ctx/inbound.rs`; item 3 also touches
`crates/nvs-server/src/serve.rs`, which item 2 above already reads.

- [ ] **The member fills the slot** — ADR 0083 § 2's two entry forms into one `Upgrade`: a path
      resolved as `spawn script`'s is, or a static method with `args:` bound by name. The row and
      the card are `crates/nvs-stdlib/src/socket.rs:155`; the body that still throws is below
      `crates/nvs-stdlib/src/socket.rs:184`'s symbol; what it fills is
      `crates/nvs-runtime/src/ctx/inbound.rs:667`'s slot, whose `fill` refuses a second
      (`crates/nvs-runtime/src/ctx/inbound.rs:746`). Three `.nvst` cases, per conventions.
- [ ] **§ 5's SSE decision, then its test** — decide whether `Core\Sse::upgrade` reaches a slot on
      a request `hyper` framed no upgrade for, and record it where the offer is gated
      (`crates/nvs-server/src/serve.rs:699`, doc at `crates/nvs-server/src/serve.rs:534`). The
      safe reading is that the slot is offered to any request and only the *hand-over* needs an
      `OnUpgrade`; write `sse_is_a_connection_isolate_with_no_receive` over whichever way it goes.
- [ ] **The retention's other half, if a session wants it** — `nvs_host::Scheduler`'s `finished`
      is now O(roots), but a *root* under `nvs serve` is never taken either
      (`crates/nvs-cli/src/serve.rs:487` spawns two and drains nothing). Two contexts for the life
      of the process is not a leak; it is the sentence the next reader of
      `crates/nvs-host/src/scheduler.rs:540` will ask about.

## Backlog

- ADR 0083 § 3's `receive()` selecting over peer and topics — `docs/adr/0083`, § 3.
- The framing slice: `101`, `tungstenite` over `NvsStream`, the socket hand-over —
  `crates/nvs-server/src/serve.rs:554`'s doc lists what it owes.
- Raw body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`.
