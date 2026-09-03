# Handoff

## State

**Goal 6, Stage 2: the `tokio` question is closed, and the listener is the whole of what is left.**
`tokio_appears_in_neither_the_manifest_nor_the_lockfile` is green in
`crates/nvs-runtime/tests/manifest_policy.rs`. The frozen name is kept and the assertion is made
stronger than the name: no manifest of ours names `tokio` outside a comment, the lock file's only
route to it is `hyper`'s, and `tokio`'s own resolved dependency list is `pin-project-lite` alone —
a runtime feature would show up there as `mio`/`socket2`/`parking_lot`, because a lock entry records
dependencies and not features. The test's own comment owns why the name reads wider than the
assertion; the workspace `Cargo.toml`'s comment above `hyper` owns the trade.

**On disk in `crates/nvs-server`: `io.rs` and nothing else.** `nvs-host` has `block_on` and
`NvsStream::poll_read`/`poll_write`. **There is no accepting half anywhere** — `net.rs` has the
stream, `from_std` and `connect`, and no listener type at all — so that is the next slice rather
than a lookup.

**The driver's acceptance sweep is truncated, and it is not a regression.** `native
examples/upload.nvs` is checked against stage 5's frozen `want`; the program legs run ahead of every
cargo check and `_check` returns at the first failure, so 55 of ~307 checks ran in session 0005.
The playbook's *Running things* bullet owns the mechanism and what not to do about it.

## Next group

**A socket to a root isolate and back.** One file set: `crates/nvs-host/src/net.rs`,
`crates/nvs-server/src/`, `crates/nvs-cli/src/main.rs`.

- [ ] **The accepting half, beside the stream in `nvs-host`.** A listener over `mio::net::TcpListener`
      that answers `Poll::Pending` after arming the readable interest, exactly as
      `crates/nvs-host/src/net.rs:538`'s `poll_read` does — ADR 0115 rule 1 puts the arming here and
      not in the adapter. `crates/nvs-host/src/net.rs:134` is the `NvsStream<S: Source>` it sits
      beside and `crates/nvs-host/src/net.rs:200` is the `from_std` an accepted socket arrives
      through.
- [ ] **`nvs serve` answers one request.** Per-core accept, one connection per coroutine,
      `hyper::server::conn::http1::Builder::serve_connection` over
      `crates/nvs-server/src/io.rs:75`'s `ConnectionIo`, driven by
      `crates/nvs-host/src/block_on.rs:101`. The module list to extend is
      `crates/nvs-server/src/lib.rs:41`; the CLI subcommand joins the match at
      `crates/nvs-cli/src/main.rs:451`. No mount table, no routing, no response policy — ADR 0097
      § 4 step 5 only.
- [ ] **The request is the root isolate of a request tree**, and it is goal 2's `Isolate`
      (`crates/nvs-host/src/isolate.rs:90`), not a second isolation path — m7.md's *Verify* makes
      Stage 9's state-bleed suite a parameterisation of one mechanism, and it proves neither if
      there are two.

## Backlog

- Stage 5's uploads are what turn `examples/upload.nvs` green — ADR 0105, and the frozen check at
  `docs/agent/loop-goal.toml:3363`.
- Until then the run has no regression floor: conformance, differential, the WSL leg and valgrind
  are all behind that short-circuit (`tools/loop.py:2032`). Whether the driver should order the
  program legs by stage is the user's call, not a session's.
- `[context] modules` named `crates/nvs-host/src/stream.rs`, which has never existed; fixed here to
  `net.rs` with `block_on.rs` added. Goal 5's list carried the same typo — worth a sweep of the
  other `docs/agent/goals/*.toml` if a pack warns again.
- ADR 0138 § 4's rejected alternative is the one to re-read before the connection loop is written.
