# Handoff

## State

**Goal 2, Stage 2. Items 4 and 5 are on disk bar one piece: the reactor, the parking stream, the
task's stack policy, the parking `connect`, deadlines — and now every wait in
`crates/nvs-host/src/net.rs` is bounded by one.** That module's doc § *Every wait is bounded by a
clock, not by a wake* is the one home for why the bound is a deadline rather than a per-call
duration and why the clock and not the resume decides; `crates/nvs-host/src/timer.rs` is unchanged
and still the one home for the timer design itself. `NvsTcp::connect_timeout` (`net.rs:192`) is ADR
0074 § 5's `connect_timeout`: the bound is on the handshake and is lifted before the stream is
handed back, so a later read takes whatever deadline its caller sets. 49 tests in the crate, green
on Windows.

**What is left of item 4 is the Unix-domain sibling**, and nothing else in Stage 2 depends on it —
items 6 (the blocking pool) and 7 (the watchdog) are independent of it and of each other.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still
not a regression** — loop-goal.md § *Stage 2* writes no `Core\Task` member until this stage is
green, and `Task::all` is Stage 4's. `nvs-host` is still outside `nvs-cli`'s dependency graph, so
`THIRD-PARTY-LICENSES.txt` still needs no regeneration.

## Next group

**The Unix-domain sibling, which is the last of loop-goal.md § *Stage 2* item 4.** One file set:
`crates/nvs-host/src/net.rs`, a new `crates/nvs-host/src/unix.rs` and
`crates/nvs-host/src/lib.rs:62-76` (the module list and the re-exports). Read `net.rs` first — the
second slice is that module with one type substituted, and the first slice decides how much of it
is copied.

- [ ] **Decide a second type against a generic over the source, and record the answer in `net.rs`'s
      module doc.** The four functions that would be shared are `wait_until_ready`
      (`net.rs:275`), `arm` (`net.rs:321`), `unregister` (`net.rs:346`) and `block_until_ready`
      (`net.rs:362`) — all four touch `self.inner` only through `mio::event::Source` plus
      `Read`/`Write`, so a generic is available. The ones that would not are `Read`/`Write`
      themselves and `finish_connecting` (`net.rs:228`), which is TCP's own question. This is a
      decided-and-recorded call under the goal's standing decisions, not an ADR.
- [ ] **`NvsUnix` over `mio::net::UnixStream`, `#[cfg(unix)]`.** ADR 0115 § 3's five-rule parking
      contract, the same deadline surface (`set_deadline`, `net.rs:126`), `from_std` and `connect`,
      exported from `lib.rs`. A Unix socket's connect completes or fails locally, so
      `finish_connecting`'s two questions are TCP's and are not carried over — say so where it is
      not carried.
- [ ] **Verify it under WSL, because Windows compiles none of it.** `verify.py` on this host never
      builds a `cfg(unix)` module, so the slice above is unverified until `wsl cargo test -p
      nvs-host` has run; `docs/agent/commands.md` § *WSL* is the invocation. Budget a call for it
      inside step 3 rather than treating the Windows run as the verdict.

## Backlog

- Item 6, the blocking pool bounded at twice the core count — ADR 0106 § 6, its own new module.
- Item 7, the watchdog reading the in-flight deadline each worker already maintains — ADR 0106 § 7.
- A `Core` spelling for a deadline is still nobody's item; ADR 0072 § 3's `{limit, deadline}` is
  Stage 3's and reaches this mechanism through `net.rs`'s `set_deadline`.
- M4's residue: the 1000-case conformance corpus count, which grows with the suite.
