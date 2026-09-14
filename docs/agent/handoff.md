# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal
opens). **Stage 3's stopping half is now on disk.** `nvs serve` arms this process's terminating
signals before anything is bound (`crate::stop::on_termination`, `crates/nvs-cli/src/serve.rs:404`),
a delivery ends in `Draining::process().begin()`, and every accept loop reads that bit on every pass
and stops accepting.

**A parked accept loop is told rather than finding out.** `serve_on_this_core` holds one
`nvs_host::wake_at_drain` registration for its whole life — not one per park, which would issue a
cross-thread handle per connection served — and `NvsAcceptor::accept_or_woken` hands the turn back
the first time the park ends, so the loop re-reads the drain instead of retrying inside `accept`.
`Draining::bit` is the crate-private accessor that makes the registration sayable; from outside the
crate a drain can still only be begun and read.

On Unix the handler does not call `begin` itself: it writes a byte to a pipe and a thread of this
process's own makes the call, because beginning a drain takes a lock and a signal delivered to the
thread holding it would deadlock the shutdown. `crates/nvs-cli/src/stop.rs`'s module doc owns that,
the `errno` it does not preserve, and why Windows needs neither the thread nor the pipe.

Left in stage 3: `sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping` — nothing
writes to `NOTIFY_SOCKET` yet, while `unit()` already renders `Type=notify`.

## Next group

**Stage 3: systemd is told each state** — one file set: `crates/nvs-cli/src/service.rs`,
`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/stop.rs`.

- [ ] **`sd_notify` says `READY`, `RELOADING`, `READY`, `STOPPING`** — the writer beside `unit()`,
      whose `Type=notify` at `crates/nvs-cli/src/service.rs:503` is what owes it
      (`rule:packaging/a-service-is-one-stored-argv`). `READY=1` goes where the boot finishes
      reporting its listeners, `crates/nvs-cli/src/serve.rs:445`; `STOPPING=1` goes in
      `crate::stop::deliver` at `crates/nvs-cli/src/stop.rs:67`, which is the one call every stop
      already ends in; the reload pair belongs to `crate::control::Process`'s reload. A datagram to
      `$NOTIFY_SOCKET` and nothing when the variable is unset, with the sink as the seam the way
      `nvs service` takes its `Manager`.
- [ ] **`WatchdogSec=30` is a promise nothing keeps** — a `Type=notify` unit with the line at
      `crates/nvs-cli/src/service.rs:516` is killed by systemd unless `WATCHDOG=1` arrives inside
      half its period. Either the pings go out beside the notifications above, or the line comes out
      of `unit()`; decide it where the writer lands, and say which in the commit.

## Backlog

- A drain does not cut a connection's idle wait short — `crates/nvs-server/src/io.rs:141`,
  `crates/nvs-server/src/socket.rs:266`; `nvs_host::wake_at_drain` is now the seam for both readers.
- A connection between one response and the next request waits under `header` and not `keepalive` —
  `crates/nvs-server/src/io.rs:130`'s `Phase::KeepAlive` looks never to be set.
- A reload's outcome is not written to `Core\Log` — `rule:config/one-local-control-socket`'s last
  sentence, in `crates/nvs-cli/src/control.rs`.
- A `[[schedule]]` waits out one interval before a stop ends its ticker —
  `crates/nvs-cli/src/serve.rs`'s `keep_ticking` closure says so.
- `crates/nvs-cli/src/stop.rs`'s Unix half is not compiled on this machine — the first Linux run of
  the suite is what checks it.
- `[context] modules` printed neither `crates/nvs-runtime/src/drain.rs` nor
  `crates/nvs-server/src/io.rs` and `socket.rs`, which are the drain's other readers.
