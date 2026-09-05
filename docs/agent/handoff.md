# Handoff

## State

**Goal 6, M7 — ADR 0083 §§ 1 and 5 are landed as far as the door.** Both cells on the
request carrier are now offered and both are read: `nvs_runtime::UpgradeSlot` only where
`hyper` framed an upgrade, `nvs_runtime::SseSlot` to **every** request the server runs
(`nvs_host::Isolate::offering_sse`, made at `crates/nvs-server/src/serve.rs:729`), and
`serve_connection` starts whichever one a request filled as a root isolate under the
connection — the same start, the same `Output::Capture`, the same join after the
connection future.

**The both-cells contradiction is decided, not deferred.** A request that filled both
asked for two responses where the connection has one: neither isolate is started, both
prepared upgrades go back through the new `nvs_runtime::Upgrade::discard` (which owns the
`unsafe` release `nvs-server`'s `forbid(unsafe_code)` cannot spell), and the peer is
answered `500`. `serve_connection`'s doc is the one home of that reading;
`nvs_runtime::SseSlot::fill` already said the decision belonged where the response is
written.

**The goal's failing acceptance check is closed** —
`sse_is_a_connection_isolate_with_no_receive` passes, over a plain `GET` with § 1's slot
absent, and `a_request_that_asks_for_both_hand_overs_is_refused` pins the refusal above.
The `-p nvs-server` fixtures are parameterised by a `Door` (`crates/nvs-server/src/serve.rs:1859`),
which decides the opening the client sends, the cell the program fills and the line the
case reads back together.

**What § 5 still owes is the response.** `crates/nvs-stdlib/src/sse.rs:47`'s § *What is
not here yet* is unchanged and now exactly true: an event stream's isolate opens and
echoes into its own capture buffer, because the `200 text/event-stream` it should be
writing into does not exist yet and `Core\Sse::current`/`send` are unregistered.

## Next group

**The response half, and the members that write into it.** Items 1 and 2 share
`crates/nvs-server/src/serve.rs` and `crates/nvs-stdlib/src/sse.rs`; item 3 is the
sibling hand-over in the first of those.

- [ ] **§ 5's `200 text/event-stream`** — ADR 0083 § 5. The stream's isolate is started
      at `crates/nvs-server/src/serve.rs:880` with `Output::Capture` and the request's
      own response is built by `crates/nvs-server/src/serve.rs:959`'s `answer`; decide
      there whether the door replaces that response and drains the isolate's capture, or
      hands the isolate a sink. Read `crates/nvs-server/src/serve.rs:567`'s § *It is not
      answered `101`* first — the same "decided rather than deferred" shape, and its last
      paragraph already names this slice.
- [ ] **`Core\Sse::current` and `send`** — ADR 0083 § 5's two remaining members, whose
      absence `crates/nvs-stdlib/src/sse.rs:47` names. The rows go in
      `crates/nvs-stdlib/src/sse.rs:63`'s `CLASS`, the symbol arm at
      `crates/nvs-stdlib/src/sse.rs:135`, and the five edits are conventions.md's. `send`
      needs item 1's body to write into, so take it after.
- [ ] **§ 1's `101` and the socket hand-over** — ADR 0083 § 1 with the goal's standing
      `tungstenite` decision. `crates/nvs-server/src/serve.rs:567` is the decision to
      overturn, and the `OnUpgrade` it says is dropped today is the one held in `offered`
      at `crates/nvs-server/src/serve.rs:865`.

## Backlog

- ADR 0083 § 3's `receive()` over peer frames and topics — waits on the framing slice.
- ADR 0083 § 4's `Core\Topic` publish/subscribe and its slow-subscriber close.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*,
  narrowed by `docs/plan/m7.md`; a decided-and-recorded call in `Core\Request`'s doc.
