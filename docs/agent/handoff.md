# Handoff

## State

**Goal 6, M7 — ADR 0083 § 2's two entry forms both open a connection now.** A path resolves as
before; a static method written `Chat::run(...)` prepares through
`crates/nvs-stdlib/src/socket.rs:399`'s `method_program`, with `args:` bound to the target's
parameters **by name** under ADR 0006 § *Decision*. The module doc's § *The method form carries its
names on the value* is the one home of how the two collapse to one prepared `Program`.

**The names ride on the callable, in a third reserved field.** `nvs_ir::lower`'s `FN_PARAM_NAMES`
writes them at a written `Class::method(...)` and at nothing else, comma-joined in declaration
order; `nvs_runtime::closure_param_names` reads them back by *name*, so an `fn` literal — whose
third field is its first capture — answers `None` rather than having a capture read as a parameter
list. That absence is load-bearing, and `crates/nvs-ir/tests/callable_names.rs` pins both halves.

**A connection is armed from the request that prepared it**, because § 1's root isolate is started
from the connection's own context, which never ran the unit: `nvs_runtime::Ctx::unit_statics` hands
the recipes over and the program installs them plus the error-class table before it calls, which is
what a path entry's `install_in` does from inside the resolver's program.

**The goal's failing acceptance check is one test closer.** `-p nvs-stdlib`'s
`an_upgrade_by_static_method_is_the_same_isolate_as_an_upgrade_by_path` passes; its three siblings —
`receive_answers_a_peer_frame_and_a_topic_delivery_from_one_wait`,
`receive_answers_null_when_the_peer_closes`, `send_throws_on_the_send_timeout_rather_than_waiting` —
all need § 3's loop, which needs a peer, which needs the framing below. Nothing about them is
misfiled: they are `Core` members of a class `crates/nvs-stdlib/src/socket.rs` owns.

**What is still open behind the door** is unchanged: `Core\Topic` (§ 4) is unregistered, and
`crates/nvs-stdlib/src/sse.rs:47`'s § *What is not here yet* still describes § 5's missing response.

## Next group

**The framing, and the three members it makes reachable.** All three share
`crates/nvs-server/src/serve.rs` and `crates/nvs-stdlib/src/socket.rs`; the first has to land before
either of the others can be written, because `receive()` has nothing to wait on without a peer.

- [ ] **§ 1's `101` and the socket hand-over** — ADR 0083 § 1, and the goal's standing decision that
      `tungstenite` is the framing crate over `NvsStream` with no adapter. The `OnUpgrade` is taken
      and held at `crates/nvs-server/src/serve.rs:715`; `crates/nvs-server/src/serve.rs:573`'s doc
      names this slice out loud and is what goes stale; the isolate is started at
      `crates/nvs-server/src/serve.rs:880` and is where the framed socket has to arrive.
- [ ] **`Core\Socket::current()` and `Socket\Message`** — ADR 0083 § 3's two shapes the loop reads:
      the connection handle a script asks for, and the `{topic, value, text}` a wait answers with.
      Rows at `crates/nvs-stdlib/src/socket.rs:191`, cards at `crates/nvs-stdlib/src/socket.rs:223`.
- [ ] **`receive()`'s one wait and `send()`'s timeout** — ADR 0083 § 3: one suspend over the peer and
      the subscribed topics, `null` when the peer closed, and a throw on the send timeout under ADR
      0074's no-unbounded-outbound-wait rule. Same two anchors as above, plus
      `crates/nvs-stdlib/src/socket.rs:399` for how a connection's own context is reached.

## Backlog

- § 5's `200 text/event-stream` — decide at `crates/nvs-server/src/serve.rs:959`'s `answer` whether
  the door replaces the response or hands the isolate a sink (ADR 0083 § 5).
- `Core\Sse::current` and `send` — ADR 0083 § 5's two remaining members, `crates/nvs-stdlib/src/sse.rs`.
- `Core\Topic::subscribe`/`publish` — ADR 0083 § 4, unregistered; `crates/nvs-stdlib/src/socket.rs`'s
  module doc owns why.
