# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is on disk, and it is the only ADR
number this goal opens). **Stage 3 is open**, and the first of its three pieces has landed: the
drain's bound and the wake that makes a connection see it. The control endpoint, `nvs ctl`, the
signal handler and `sd_notify` are not written, so all three of stage 3's checks are still red —
the one the driver reports, `nvs ctl --help`, is an item still open rather than a regression.

On disk now: `[server] drain_timeout` resolves into `nvs_config::Waits::drain` (`"30s"` unwritten,
`false` and `0` refused under `E0619` like the four waits), and `nvs_runtime::Drain` carries a wake
registry — `Drain::wake_at_drain` takes a `Send` closure and hands back a `DrainWake` that
deregisters on drop, and `Drain::begin` fires every one of them outside the lock.

**Two things not to re-derive.** The period lives in `nvs_config::Waits`, which already travels to
every connection as `nvs_server::bounds::Connection::drain` does one door over — **not** beside the
bit in `nvs-runtime`, which holds the bit only because two crates that cannot see each other read
it, and that is not true of a number a connection is already handed. And the registered wake is
`Send` where `nvs_runtime::host::Waker` deliberately is not, because the thread that took the signal
is not the core the task is parked on; the host supplies the closure, so `nvs-host`'s `RemoteWake`
is what goes inside one.

**Nothing registers a wake yet.** The registry is asserted in `crates/nvs-runtime/src/drain.rs`'s
own tests and has no caller until the accept loop takes one per connection, which is the first
backlog item and what `a_terminating_signal_drains_serve_and_every_connection_closes_cleanly` rests
on.

## Next group

**Stage 3: the control endpoint's three operations** — one file set:
`crates/nvs-config/src/control.rs`, `crates/nvs-server/src/control.rs`, `crates/nvs-cli/src/serve.rs`.

- [ ] **`Operation`'s three arms, answered over HTTP on the socket** — `reload`, `config` and
      `status` at `crates/nvs-server/src/control.rs:26`, each answer carrying the server version and
      a mismatch refused (`rule:config/one-local-control-socket`), and `config` taken from the live
      snapshot with each directive's origin
      (`rule:config/ctl-config-reports-the-live-snapshot`). Framing is `hyper`'s on both halves and
      no control operation runs Novis code.
- [ ] **One endpoint, one thread, one operation at a time** — the blocking accept over
      `crates/nvs-config/src/control.rs:133`'s `Endpoint`, so two reloads on one endpoint are
      answered one after the other and `status` reports the in-flight count and the drain bit. A
      thread and not a runtime (`rule:concurrency/one-scheduler`).
- [ ] **`serve` binds it before any listener accepts, and refuses a writable directory** — at
      `crates/nvs-cli/src/serve.rs:519`, beside the `nvs_server::Draining::process()` the worker
      already takes at `crates/nvs-cli/src/serve.rs:551`; the directory check is the one every
      configuration file gets (`rule:config/one-local-control-socket`).

## Backlog

- The drain wake has no caller: register one per connection task at
  `crates/nvs-server/src/serve.rs:1726` and arm `Waits::drain` at the next wait,
  `crates/nvs-server/src/io.rs:133` — stage 3's nvs-cli signal test needs it.
- `nvs ctl`, the signal handler and `sd_notify`: a new `crates/nvs-cli/src/ctl.rs` wired from
  `enum Command` at `crates/nvs-cli/src/main.rs:235` — the check the driver currently reports.
- `docs/agent/loop-goal.toml`'s `[context] modules` did not name `crates/nvs-server/src/io.rs` or
  `crates/nvs-config/src/tree.rs`, both of which a `[server]` directive has to be added to.
