# Handoff

## State

**Goal `websocket-client` — stage 5, the conversation, is done.** All six of the stage's
`cargo test -p nvs-stdlib` names pass; its two `.nvst` cases and
`crates/nvs-config/tests/directives.rs:277` were already green, so the stage's three checks should be
too. Stages 1–4 stay closed.

**What the conversation does now.** `Open` carries `idle`, `until`, the send wait, `ping` and the
read mark, and `receive` arms each wait with whichever instant is nearest; a peer's ping or pong
restarts the silence, and a wait that ended at the ping's asks whether the peer is there. A message
past `maxMessage` closes with `1009` and throws naming the cap, `close` sends the program's code and
reason, and `Open`'s `Drop` sends `1001`. A failure read after this end has closed answers `null`.

**The one divergence between this class's cards and its code**: `CLOSE_DOC` says a `close` waits for
the peer's own close under the send wait (`crates/nvs-stdlib/src/http/socket.rs:361`) and it does not
— the frame goes out and the connection is let go.

## Next group

**Stage 6: the rulebook** — one file set: `docs/rules/http-server.json`, `docs/rules/testing.json`,
and what `python tools/rules.py --render` rewrites from them. Each item is one rule's `status`, which
is `designed` today and which `python tools/rules.py --show <id>` is what the check reads; every
sentence of a rule is what `shipped` claims, so each item is that judgement and not a flag.

- [ ] **The two bounds-and-lifetime rules stage 5 implemented** — `docs/rules/http-server.json:1337`
      and `docs/rules/http-server.json:1350`. The read loop that arms every bound is
      `crates/nvs-stdlib/src/http/socket.rs:726`, the `1001` is `Open`'s `Drop` above it, and the
      `1009` and the second-`receive` refusal are its two named faults.
- [ ] **`an-outbound-socket-is-opened-like-an-outbound-call`** — `docs/rules/http-server.json:1322`.
      The row, the `ws`/`wss` roster and "never pooled" are stages 1–4's, landed and tested in
      `crates/nvs-stdlib/src/http/transport.rs:1713`.
- [ ] **`testing/an-outbound-socket-is-answered-by-a-scripted-peer`** — `docs/rules/testing.json:229`.
      The scripted arm is `crates/nvs-stdlib/src/http/socket.rs:458`, and the `.nvst` cases under
      `tests/conformance/core/http-socket-*` are what it is guarded by.
- [ ] **Re-render the rulebook** — `tools/rules.py:63` writes `docs/rules/*.md` and
      `docs/ground-rules.md` from the JSON, so a status flipped by hand leaves both stale;
      `python tools/rules.py --render --check` is the gate that says so.

## Backlog

- `close` does not wait for the peer's close under the send wait, which `CLOSE_DOC` says it does —
  `crates/nvs-stdlib/src/http/socket.rs:361` is the card that has to change or be met.
- `Core\Http\Options`' `ping` reaches no `.nvst` case: the conversation is only reachable against a
  live host, so the Rust case at `crates/nvs-stdlib/src/http/socket.rs:1176` is its whole coverage.
- ADR 0183 § 10's memory price is stated, never asserted — no case pins a socket's cost to the task.
- `permessage-deflate`, RFC 8441, reconnection and the subprotocol libraries stay out of this goal
  (`docs/agent/loop-goal.md` § *Standing decisions*).
