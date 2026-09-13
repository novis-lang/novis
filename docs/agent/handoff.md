# Handoff

## State

**Goal `websocket-client` is met.** Stage 6 flipped its four rules to `shipped` with `guardedBy`
filled from this goal's cases and tests, and `python tools/rules.py --render` rewrote the three
generated files. Stages 1–5 were already green, so every `[[check]]` in `docs/agent/loop-goal.toml`
now passes.

**One divergence stage 5 left is closed**: `close` sends its frame and then waits for the peer's own
under the send wait, which is what ADR 0183 § 5 and the member's card both say
(`crates/nvs-stdlib/src/http/socket.rs:937`). A peer that never answers is still closed, and no exit
from that wait throws.

**One defect this goal found and did not fix**, because it belongs to another rule and to every
`Core` class at once: `rule:classes/graph-copy` states that an object holding a host handle is
refused at a copy boundary, and no carrier makes that refusal — a `Core\Http\Socket` handed to
`spawn script … with(args:)` arrives in the child with every slot intact, and `Core\Serialize::encode`
takes one too. It is written up as `crates/nvs-runtime/src/graph.rs` gap 3 and carried in
[carried-gaps.md](carried-gaps.md) § *Unowned*, where the decision it waits on is named. Nothing is
shared by the copy — a handle table belongs to one `Ctx` — but the copied key indexes the receiving
side's table, so it reads whatever that side opened at that index.

## Next group

**The goal is closed, so the next group is the chain's next goal, not a stage of this one.** If the
driver has not switched yet, this is the one slice left in the file set this goal loaded, and it needs
the user's answer before any of it is written:

- [ ] **`rule:classes/graph-copy`'s host-handle refusal, which nothing implements** —
      `crates/nvs-runtime/src/graph.rs:59` gap 3, refused where
      `crates/nvs-runtime/src/graph.rs:371`'s `refusable` refuses a closure. What has to be decided is
      where the mark saying a class holds a handle lives: a bit on the `ClassDesc`, which is the same
      representation question gap 1 asks for a closure, or the declared type at the copy site.

## Backlog

- `crates/nvs-runtime/src/graph.rs` gap 3 — carried in [carried-gaps.md](carried-gaps.md) § *Unowned*.
- `permessage-deflate`, RFC 8441, reconnecting and the subprotocol libraries stay out of `Core` — goal
  `websocket-client` § *Standing decisions* is their one home.
