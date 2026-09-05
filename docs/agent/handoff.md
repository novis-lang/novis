# Handoff

## State

**Goal 6, M7 — ADR 0083 § 4's bus is whole across cores.** `Core\Topic::publish`
(`crates/nvs-stdlib/src/topic.rs:527`) fans out to the subscribers on its own core by ADR 0023
§ 2's graph copy, then hands ADR 0023 § 2's *encoding* of the same value to every other core that
reported a subscriber, and answers both. The transport is `crates/nvs-stdlib/src/bus.rs`: a
process-wide registry of per-core mailboxes, each holding a bounded queue of envelopes and the
subscriber count its own core last saw per topic. A core registers itself on first use and its
entry goes with the thread, so `nvs-server` needs no edge to this crate for § 4 to hold across its
cores. `Core\Socket::receive()` drains the mailbox before it reads either source
(`crates/nvs-stdlib/src/socket.rs:1001`), which is where the bytes become one value per subscriber.
The four decisions behind it — bytes rather than a value, what the count means and what it
over-reports, lazy registration, a refused overflow — live in those two modules' own docs.

**Two known gaps, both written down where they live.** The wake seam
(`nvs_runtime::Ctx::deliver`'s known gap) is unchanged and a cross-core delivery inherits it rather
than adding one, because the drain runs at the same `receive()`. The per-subscriber bound and § 4's
"a slow subscriber is closed" are item 1 below and are the last open work in § 4.

**The goal's failing acceptance check was filed where it could not run.** Every name in
`ADR 0083 § 4 -- Core\Topic, across every core` is a claim about `Core\Topic::publish`, and
`crates/nvs-server/Cargo.toml` does not name `nvs-stdlib`; the check is now `-p nvs-stdlib` in both
`docs/agent/loop-goal.toml` and `docs/agent/goals/6-server.toml`, with a comment saying why, and
three of its four names are landed. The fourth is item 1.

## Next group

**What § 4 still owes, and the seam under it.** Both share `crates/nvs-stdlib/src/topic.rs` and
`crates/nvs-runtime/src/peer.rs`; item 1 closes the goal's last open name in the § 4 check.

- [ ] **The bounded queue, and closing a slow subscriber** — ADR 0083 § 4's priority-1 rule: each
      subscriber's queue is capped, and when it overflows *that subscriber's connection* is closed
      with a defined code and a metric increments, while the publisher is never blocked. The cap
      belongs on `crates/nvs-runtime/src/peer.rs:142`'s `Inbox::push`, which today takes anything;
      the count and the decision belong in the fan-out at
      `crates/nvs-stdlib/src/topic.rs:527`. **The design question to settle first is who closes**:
      the peer is on the subscriber's own isolate (`crates/nvs-runtime/src/ctx/isolate.rs:82` is
      where a context and its socket meet) and may be on another core entirely, so the publisher
      cannot reach it — the likely shape is a flag the overflowing queue carries and the
      subscriber's next `receive()` obeys, which is `crates/nvs-stdlib/src/socket.rs:1001`. Landing
      it makes `a_subscriber_that_never_reads_is_closed_and_the_publisher_is_unaffected` real.
      `crates/nvs-stdlib/src/bus.rs:234`'s own refusal — a full mailbox drops an envelope and
      counts nobody for it — is the same rule one layer down and should be decided with it.
- [ ] **The wake seam: a park over both sources** — a delivery queued while its connection is
      already parked inside `receive()` waits for the *next* `receive()`.
      `crates/nvs-runtime/src/ctx/isolate.rs:82`'s known gap is the specification;
      `crates/nvs-stdlib/src/socket.rs:1001` is the member that parks and
      `crates/nvs-server/src/socket.rs:1` is the framing side that has to take part, since the park
      is inside the codec's read. ADR 0083 § 3's "one wait over two sources" is what it owes.

## Backlog

- ADR 0083 § 7's bounds, budgets, reload and drain — its own check, stage 6b (`loop-goal.toml`).
- `nvs-types`' three qualifier cases at the connection boundary — `tainted` payload, `tainted`
  topic name, `secret` through `upgrade`/`publish` (stage 6b's `-p nvs-types` check).
- A published object crossing cores resolves against the *draining* program's class table, not the
  publisher's — `nvs_runtime::graph`'s known gap 2, restated in `topic.rs`'s drain.
- `[context] modules` prints no `nvs-stdlib/src/topic.rs` or `socket.rs` line, so 6b's own two
  modules are absent from the map; `[context] adrs` gained `0083 §4` this session.
