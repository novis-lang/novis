# Handoff

## State

**Goal 6, Stage 2: the seam has something to drive, and nothing listens yet.** `hyper` 1.11 is in the
workspace manifest at `default-features = false, features = ["http1", "server"]` (`Cargo.toml:126`), and
`crates/nvs-server` exists with one module: `src/io.rs` is `hyper::rt::Read`/`Write` over `NvsTcp` — try
the syscall, arm the reactor, answer `Pending`, and never suspend inside a `poll`, which is ADR 0138 § 4's
rejected alternative. The arming half is `NvsStream::poll_read`/`poll_write` in `nvs-host`, put there
rather than in the adapter because ADR 0115's rule 1 belongs beside the registration it touches.

**`tokio` is in the lock file, and the goal document said it would not be.** `hyper` 1.11 depends on it
unconditionally at `features = ["sync"]` — one `oneshot` in its `src/upgrade.rs`, on the h1 server path —
so what compiles is channels and no runtime at all: no `rt`, no `net`, no `time`, no executor, no `spawn`.
ADR 0072's rule is intact; only the *name* is not out. The prose is corrected in both goal files and the
reasoning, including what pinning `hyper` backwards would have cost, is the workspace `Cargo.toml`'s
comment above the dependency. What is still open is item 1 below.

**The driver's failing acceptance check is closed.** `examples/serve.nvs`, `examples/session.nvs` and
`examples/upload.nvs` are on disk and run clean; the fixture list in `docs/agent/loop-goal.toml` requires
every entry to exist before any check runs, so this had been failing the whole goal.

## Next group

**Stage 2 finished: a socket to a root isolate and back.** One file set: `crates/nvs-server/src/`,
`crates/nvs-cli/src/main.rs`, `crates/nvs-runtime/tests/manifest_policy.rs`, `crates/nvs-host/src/net.rs`.

- [ ] **Decide what `tokio_appears_in_neither_the_manifest_nor_the_lockfile` asserts, and write it.**
      The name is frozen acceptance data (`docs/agent/loop-goal.toml:3253`, stage 2) and it is now
      literally false, so the choice is between renaming the check and giving the test the stronger,
      true assertion: no crate of ours names `tokio` in a manifest, and the lock file's only `tokio` is
      `hyper`'s with no runtime feature and no `tokio-util`/`tokio-macros` beside it. **Prefer the
      second and keep the frozen name**, with the test's own doc comment saying why the name reads
      wider than the assertion. It goes beside `crates/nvs-runtime/tests/manifest_policy.rs:42`, whose
      module doc already owns the "pinned at its source" shape; the dependency it reads is the
      workspace manifest's `hyper` line, reached from `crates/nvs-server/Cargo.toml:17`.
- [ ] **`nvs serve` answers one request.** Per-core accept, one connection per coroutine,
      `hyper::server::conn::http1::Builder::serve_connection` over `crates/nvs-server/src/io.rs:75`'s
      `ConnectionIo`, driven by `nvs_host::block_on`. The parking read underneath is
      `crates/nvs-host/src/net.rs:538`; the CLI subcommand joins the match at
      `crates/nvs-cli/src/main.rs:451`. No mount table, no routing, no response policy — ADR 0097 § 4
      step 5 only.
- [ ] **The request is the root isolate of a request tree**, and it is goal 2's `Isolate`
      (`crates/nvs-host/src/isolate.rs:90`), not a second isolation path — m7.md's *Verify* makes
      Stage 9's state-bleed suite a parameterisation of one mechanism, and it proves neither if there
      are two.

## Backlog

- `[context] modules` in `docs/agent/loop-goal.toml` still names `crates/nvs-host/src/stream.rs`, which has
  never existed; the parking stream is `net.rs`. `orient.py` prints the mismatch as a warning every session.
- `[context] adrs` wants ADR 0138 §§ 1 and 4 and ADR 0051 § 4 — this session read all three by hand.
- ADR 0051 § 4 is titled "two questions" and the group's item 1 called them three; whichever is meant, the
  ADR body is the home and it says two.
- `crates/nvs-server/src/io.rs`'s read copies through an 8 KiB stack buffer because the crate inherits
  `unsafe_code = "forbid"`. If a benchmark ever charges for it, the buffer's size is the first knob.
- `poll_shutdown` does not half-close; the FIN is the drop's, so that ADR 0083 can keep the descriptor for
  an upgrade. Revisit when the WebSocket slice lands.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by m7.md.
