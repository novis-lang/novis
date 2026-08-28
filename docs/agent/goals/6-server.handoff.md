# Handoff

## State

**Goal 6 — the last goal of the parity program — has just started; nothing of it has landed yet.** M4 and
goals 1–5 reached their whole acceptance lists and all six are now this goal's Stage 1 floor. That floor
matters more here than anywhere else in the chain: a listener is where an old assumption about isolation,
capabilities or the graph copy gets its first adversarial traffic.

**`crates/nvs-server` does not exist yet.** Two things every session holds:

- **`hyper` runs with no async runtime.** `default-features = false, features = ["http1", "server"]` brings
  no `tokio`; h1 needs no `Executor` and `serve_connection` spawns nothing, so the connection future is
  driven by a `block_on` on the coroutine that owns the connection, over `hyper::rt` adapters wrapping
  goal 2's parking stream. One polled future per connection is not a second scheduler.
- **A filesystem path is never derived from a URL at request time.** ADR 0097 § 2 is the governing rule and
  § 4's five-step resolution is how it is kept. The test that says it holds is Stage 9's set equality, not
  a traversal suite.

Goal 5's containers are still up — the session store and the fleet lease both need Redis, and
`#[Test(db:)]` needs a database.

## Next group

**Stage 2: a socket to a root isolate and back, and nothing else.** No mount table, no routing, no response
policy — those are Stages 3 and 4, and building them into the first connection is how the seam ends up
untestable.

One file set: `crates/nvs-server/src/`, `crates/nvs-host/src/stream.rs`,
`crates/nvs-host/src/isolate.rs`.

- [ ] **The `block_on` seam**, which carries this goal's one pre-authorized ADR slot and is its first
      slice. How a `hyper` connection future is driven from a coroutine, what the waker does, what happens
      when the future wakes on a core other than the one that parked it, and why this is not an executor.
      Write the ADR, then the code; the number comes from `python tools/brief.py`, re-checked immediately
      before the file is created.
- [ ] **`nvs serve` answers one request**, per-core accept and dispatch, a connection on a coroutine.
- [ ] **The request is the root isolate of a request tree** — goal 2's `Isolate`, not a second isolation
      path. This is stated as its own item rather than assumed because Stage 9's state-bleed suite is a
      *parameterisation* over one mechanism; if it ends up two suites, this item was not done.
- [ ] **`tokio_appears_in_neither_the_manifest_nor_the_lockfile` still passes** with `hyper` in the tree.
      The claim has always been about a runtime rather than about the `Future` trait, and ADR 0099's own
      bullet now says so — this is the check that keeps the distinction honest rather than assumed.

## Backlog

- Raw/unparsed body access for an arbitrary content-type is an open gap ADR 0024's *Revisiting* flags,
  narrowed by m7.md to what `body()` and `bodyStream()` do not already answer. Decided-and-recorded in
  `Core\Request`'s module doc if it comes up — not a new ADR and not a `BLOCKED`.
- No TLS listener and no h2c: ADR 0097 § 1 dropped both and a proxy terminates TLS.
- When Stage 9's last check goes green — `check-migration.py` at 100% — **the parity program is finished**
  and the chain has no next goal. The milestone table's order 6 is M4B, whose staged goal is
  `docs/agent/next-goal-m4b.md`.
