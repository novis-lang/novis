# Handoff

## State

**Goal `event-streams` — door two is complete end to end, and stage 3 is landed.**
`Core\Sse::stream()` opens the same body cell `Core\Response::stream` opens, declares
`text/event-stream` with the two headers an event stream does not survive a proxy without, and
refuses a request that had already set a status. `Core\Sse->send` frames through
`crates/nvs-runtime/src/sse.rs:119`'s `Event` and writes into whichever half of the context holds a
body, so off a connection both members are inert and a `.nvst` case can freeze a whole event stream.
`Core\Sse` is now a contextual class — members, no slots — beside `Core\Socket`.

**Stage 2's three acceptance checks named `-p nvs-server` after the framing moved to `nvs-runtime`,
and are re-pointed in both toml copies.** Nine passing tests were being reported as "did not run";
the playbook bullet is the trap. The stage 3 case list named a `reject/` case for the status
contradiction, and the goal's § *Standing decisions* makes every refusal on this door a `LogicError`
— a throw a program catches, never a diagnostic — so the claim is pinned in `core/` and the check
names it there.

**Record `docs/decisions/0176.md` is still open** with the stage 0 correction, and owes the framing's
crate as well.

## Next group

**Stage 4: door one, and the classification its parameters carry** — one file set:
`crates/nvs-stdlib/src/sse.rs`, `crates/nvs-stdlib/src/socket.rs` and
`crates/nvs-types/tests/sockets.rs`.

- [ ] **`Core\Sse::current(): Core\Sse`** — the five edits at `crates/nvs-stdlib/src/sse.rs:120`,
      modelled on `crates/nvs-stdlib/src/socket.rs:963`, whose whole body is one question of the
      context. The question here is the one to settle first and it is not `has_peer`: door two's
      handle is the same type, so *is a body stream open* would answer `true` inside an ordinary
      streaming response, and what the refusal has to name is an isolate that is not an event stream
      at all. `rule:concurrency/two-doors-one-isolate`.
- [ ] **The three classification tests**, in `crates/nvs-types/tests/sockets.rs` beside the socket
      door's — `a_tainted_event_name_is_refused_at_the_call_site`, `a_tainted_id_is_refused_at_the_call_site`
      and `a_tainted_data_payload_is_accepted_and_stays_tainted`, over the `send` row at
      `crates/nvs-stdlib/src/sse.rs:150`, whose two sinks are `CoreTy::Nullable(&CoreTy::Text(Qual::Sink))`.
      `rule:security/unclassified-parameter-refuses-tainted`.
- [ ] **`Core\Sse->retry(Core\Time\Duration $after): void`** — the goal's stage 4 item 17, one
      `retry:` line through `crates/nvs-runtime/src/sse.rs:95`'s field, written beside `send` at
      `crates/nvs-stdlib/src/sse.rs:429`. The `Duration` reader is
      `crates/nvs-stdlib/src/time.rs`'s. `rule:concurrency/connection-bounds-are-finite`.

## Backlog

- `Core\Sse->receive()` and the `Core\Sse\Message` class, topics only — goal prose stage 4 item 16.
- Wiring door one where the cell is already taken — goal prose stage 4 item 18, `crates/nvs-server/src/serve.rs:936`.
- The bounds and the heartbeat derived from the write wait — goal prose stage 5.
- `docs/decisions/0176.md` owes the stage 0 correction and the framing's crate — goal prose stage 6.
