# Handoff

## State

**Goal 6, Stage 2: the `block_on` seam has landed and the rest of the stage has not.**
[ADR 0138](../adr/0138-a-connection-future-is-driven-by-the-coroutine-that-owns-it.md) is the goal's one
pre-authorized ADR slot and it is now spent; `crates/nvs-host/src/block_on.rs` is its implementation, green
under `verify.py -p nvs-host` with five tests — the ordering that makes a lost wakeup impossible, a
cross-thread wake into a parked core, one permission per drive, and both halves of a cancellation.

**`crates/nvs-server` still does not exist and `hyper` is not in the manifest.** The seam deliberately
needs neither: it is `std::task` and this crate's own reactor, so item 2 adds the dependency and the
adapters over it rather than around it.

A filesystem path is never derived from a URL at request time — ADR 0097 § 2 governs, § 4's five steps are
how it is kept, and Stage 9's set equality is the test that says so. Goal 5's containers are still up (the
session store and the fleet lease need Redis, `#[Test(db:)]` needs a database).

**`orient.py`'s pack was missing `crates/nvs-host/src/net.rs`**: `[context] modules` in
`docs/agent/loop-goal.toml` names `crates/nvs-host/src/stream.rs`, which has never existed — the parking
stream is `net.rs`, and the tool printed the mismatch as a warning twice. That entry wants replacing, and
`crates/nvs-host/src/{scheduler,reactor,blocking}.rs` adding beside it: the seam and everything Stage 2
builds on it sit on those three.

## Next group

**Stage 2 finished: a socket to a root isolate and back.** Still no mount table, no routing and no response
policy — Stages 3 and 4.

One file set: `crates/nvs-server/src/` (new), `Cargo.toml`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-host/src/net.rs`.

- [ ] **`hyper` into the workspace manifest and the `hyper::rt` adapters over the parking stream.**
      `default-features = false, features = ["http1", "server"]`, with ADR 0051 § 4's three questions
      answered in a workspace-`Cargo.toml` comment beside `mio`'s, which is that file's line 84. The
      `Read`/`Write` adapters wrap `crates/nvs-host/src/net.rs:130`'s `NvsStream` and return `Pending`
      rather than suspending inside a poll, which is ADR 0138's rejected alternative and the deadlock it
      names; the drive is `crates/nvs-host/src/block_on.rs:101`.
- [ ] **`nvs serve` answers one request.** Per-core accept, one connection per coroutine,
      `serve_connection` under `block_on`, no executor installed — the `Command` arm goes at
      `crates/nvs-cli/src/main.rs:452`.
- [ ] **The request is the root isolate of a request tree** — goal 2's `Isolate`
      (`crates/nvs-host/src/isolate.rs:1045`), never a second isolation path, because Stage 9's state-bleed
      suite is meaningless otherwise.
- [ ] **`tokio_appears_in_neither_the_manifest_nor_the_lockfile`** still passes with `hyper` in the tree.
      The test does not exist yet and needs a crate that can host it; how a test reaches the workspace
      manifest and lockfile from inside one is `crates/nvs-cli/build.rs:123`.

## Backlog

- Fix `[context] modules` in `docs/agent/loop-goal.toml`: `stream.rs` for `net.rs`, plus `scheduler.rs`,
  `reactor.rs`, `blocking.rs` — `docs/agent/loop-goal.toml`.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by
  `docs/plan/m7.md`.
- Stage 9's state-bleed suite across an isolate boundary — `docs/plan/m7.md`'s *Verify*.
- `wrk`/`oha` throughput against PHP 8.5 + FPM, recorded in `benches/` — `docs/plan/m7.md`'s *Verify*.
