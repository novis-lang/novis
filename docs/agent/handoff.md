# Handoff

## State

**Goal 6, M7 — ADR 0083 § 5 is landed as far as the door.** The request carrier now holds
**two** cells: `nvs_runtime::UpgradeSlot` for § 1's socket hand-over and
`nvs_runtime::SseSlot` for § 5's, distinct types because a shared slot would be one the
connection has to ask the *kind* of. `Core\Sse` is registered with § 2's identical clause —
`upgrade(entry, args)`, a path or a static method, `void` — and its body fills the second
cell. `crates/nvs-stdlib/src/sse.rs`'s module doc is the one home of what differs from the
sibling door; everything behind the door is `crate::socket`'s and shared outright
(`entry_program`, `retained`, `release_crossed` are now `pub(crate)` and `entry_program`
takes the member's own spelling for its refusals).

**Nothing offers the cell yet**, so `Core\Sse::upgrade` throws everywhere today and
`sse_is_a_connection_isolate_with_no_receive` is still the goal's one failing acceptance
check. `nvs_runtime::Inbound::offer_sse` is the door's half and has no caller.

**A design question is open and item 1 below is where it lands.** A WebSocket handshake
request can fill *both* cells, and `serve_connection` holds room for one connection isolate
(`crates/nvs-server/src/serve.rs:825`). Neither of the two obvious repairs is free: dropping
the losing `nvs_runtime::Upgrade` leaks its argument reference (that type's own doc says so)
and `crates/nvs-server` is `forbid(unsafe_code)`, so it cannot release one; and offering the
SSE cell only to a non-upgradable request contradicts § 5's "offered to **every** request".
The safe reading is a `Upgrade::discard(self)` in `nvs-runtime`, which owns the unsafe, so
the door can refuse the contradiction with a `500` where the response is written — which is
where § 5 says it belongs. Decide it in the item, do not re-open the ADR.

## Next group

**The door, and the isolate it starts.** All three touch
`crates/nvs-server/src/serve.rs`; items 1 and 2 also touch
`crates/nvs-runtime/src/ctx/inbound.rs` and `crates/nvs-host/src/isolate.rs`.

- [ ] **Offer the cell to every request** — ADR 0083 § 5. A `nvs_runtime::SseSlot::new()`
      beside `crates/nvs-server/src/serve.rs:697`'s `upgradable` (which stays the *upgrade*
      question), an `offering_sse` beside `crates/nvs-host/src/isolate.rs:211`, and the
      offer made in the `Reply::Run` arm at `crates/nvs-server/src/serve.rs:737` where the
      slot's already is. Settle the both-cells contradiction here — `## State` above names
      the reading and the one `nvs-runtime` addition it needs
      (`crates/nvs-runtime/src/ctx/inbound.rs:716`).
- [ ] **Start what the cell holds, and the failing check** — ADR 0083 §§ 1 and 5. The
      sibling block is `crates/nvs-server/src/serve.rs:820`; a root isolate under *this*
      connection, `Output::Capture`, joined the same way. Then
      `sse_is_a_connection_isolate_with_no_receive`: the fixture to reach for is
      `crates/nvs-server/src/serve.rs:1725`'s `upgrade_leaving` (parameterise which cell it
      fills — four call sites) and `crates/nvs-server/src/serve.rs:1789`'s `upgrade_once`,
      which hardcodes an RFC 6455 handshake and a `"{path} upgraded"` needle where an SSE
      case needs a plain `GET` and its own.
- [ ] **The response half** — ADR 0083 § 5's `200 text/event-stream`, whose body the isolate
      writes while the connection future runs. `crates/nvs-server/src/serve.rs:554`'s § *It
      is not answered `101`* is the sibling decision to read first: the same "decided rather
      than deferred" reasoning applies, and `crates/nvs-stdlib/src/sse.rs`'s § *What is not
      here yet* names the members (`Core\Sse::current`, `send`) this waits on.

## Backlog

- The method entry form still throws at both doors, waiting on `CLOSURE_PARAM_NAMES_SLOT` —
  `crates/nvs-stdlib/src/socket.rs` § *The method form waits on a name*.
- `Core\Topic` (§ 4) is unregistered — `crates/nvs-stdlib/src/socket.rs`'s module doc.
- No `101` and no framing behind § 1's upgrade — `crates/nvs-server/src/serve.rs:554`.
