# Handoff

## State

**Goal 6, Stage 2: a socket to an h1 response and back, on one core, inside `nvs-server`.** Two unit
tests in `crates/nvs-server/src/serve.rs` drive it end to end — one request over a closing
connection, and a second request over a kept-alive one, which is the seam's own question because it
only passes if the connection future parked and the reactor resumed the drive.

**On disk.** `nvs_host::NvsListener` (`crates/nvs-host/src/net.rs:293`) is the accepting half: a
newtype over `NvsStream<mio::net::TcpListener>`, so the four waiting functions still have one home,
with `accept` (parks) and `poll_accept` (arms, then `Pending`). `nvs_server::serve` is the loop:
accept, one child task per connection, `hyper` `serve_connection` under `block_on`, and a shutdown
that waits its connections out. `nvs-server` now depends on `nvs-runtime` for the three names that
spell a task.

**The handler cannot park, and that is the next slice's whole problem.** `serve_connection`'s handler
runs inside the connection future's poll, so it is a `Fn(Request) -> Response<Answer>` today.
ADR 0006's isolate is a *task*, which is what a park needs — so the request slice has to decide
whether the isolate is spawned and awaited around the poll, or the response is assembled before
`hyper` is told about it. Nothing in the tree assumes either yet.

**Nothing user-reachable starts any of this.** There is no `nvs serve` subcommand — the only hits for
`serve` in `crates/nvs-cli/src/main.rs` are comments — and no connection carries a deadline
(ADR 0074 § 5), which is why the CLI slice and the deadline slice belong together.

**The driver's acceptance sweep is truncated, and it is not a regression.** `native
examples/upload.nvs` is checked against stage 5's frozen `want`; the program legs run ahead of every
cargo check and `_check` returns at the first failure. The playbook's *Running things* bullet owns
the mechanism and what not to do about it.

## Next group

**A request that runs, and a server you can start.** One file set: `crates/nvs-server/src/serve.rs`,
`crates/nvs-cli/src/main.rs`, `crates/nvs-host/src/isolate.rs`.

- [ ] **The request is the root isolate of a request tree**, and it is goal 2's `Isolate`
      (`crates/nvs-host/src/isolate.rs:90`), not a second isolation path — m7.md's *Verify* makes
      Stage 9's state-bleed suite a parameterisation of one mechanism, and it proves neither if there
      are two. The seam to change is the handler bound at
      `crates/nvs-server/src/serve.rs:138`; `crates/nvs-server/src/serve.rs:76`'s `Answer` is the
      body the isolate's output becomes, and ADR 0088 § 3's table is why `echo` is what fills it.
- [ ] **`nvs serve` starts one core and runs the loop.** The subcommand joins the enum at
      `crates/nvs-cli/src/main.rs:141` and the match at `crates/nvs-cli/src/main.rs:451`; the
      scheduler-and-reactor boot to copy is `crates/nvs-cli/src/main.rs:1055`, and what it spawns as
      its root task is `crates/nvs-server/src/serve.rs:171`'s `serve_on_this_core` with
      `ControlFlow::Continue(())`. ADR 0097 § 4 step 5 only — no mount table.
- [ ] **A connection is bounded by a clock.** ADR 0074 § 5's waits, set through
      `crates/nvs-server/src/serve.rs:138` on the stream `nvs_server::ConnectionIo::stream_mut` hands
      out, using `crates/nvs-host/src/net.rs:178`'s `set_deadline`. A `[server]` block is where the
      numbers come from, so this lands with the CLI slice or immediately after it.

## Backlog

- Per-core accept fan-out: one bound socket, `NvsListener::from_std` per core — docs/plan/m7.md.
- `nvs_server::serve` logs nothing when a connection fails; the log tier is ADR 0020 § 4's `[log]`.
- `Answer` is one frame; a streamed response body is ADR 0105's `bodyStream()` side — docs/plan/m7.md.
- The `[server]` block itself — listen address, `static`, `dispatch` — ADR 0097 §§ 3-4.
- Raw/unparsed body access for an arbitrary content-type stays an open gap — ADR 0024 *Revisiting*.
