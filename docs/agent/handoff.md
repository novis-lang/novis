# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal
opens). **Stage 3's reporting half is on disk**: a `Type=notify` unit gets `READY=1` once every
listener is bound, `RELOADING=1` and `READY=1` around every reload, and `STOPPING=1` once the drain
has begun.

**The writer is hand-written and takes no crate.** `crate::service::Notify` is one datagram of
`NAME=value` lines to whatever `$NOTIFY_SOCKET` names, silence where the variable is unset, and a
`Supervisor` sink as the seam — which is what makes the states assertable on a box with no systemd.
`service.rs`'s module doc § *The manager is told what state this process is in* owns the rest;
`rule:packaging/a-service-is-operator-surface` no longer claims a crate for it.

**The process's manager is installed by the boot**, because the reporter that needs it is handed
nothing: `crate::stop::deliver` takes `Notify::process()` exactly as it takes
`nvs_server::Draining::process()`. `deliver_to` is the same stop over a drain a caller names, which
is how a case reports a stop without spending the one process-wide drain bit the signal case owns.

Left in stage 3: `WatchdogSec=30` is still a promise nothing keeps.

## Next group

**Stage 3: the watchdog line, and the reload's log** — one file set: `crates/nvs-cli/src/service.rs`,
`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/control.rs`.

- [ ] **`WatchdogSec=30` is a promise nothing keeps** — the line `unit()` renders at
      `crates/nvs-cli/src/service.rs:538` kills a `Type=notify` service unless `WATCHDOG=1` arrives
      inside half its period. `rule:packaging/the-generated-unit-is-hardened` says the ping comes
      from the accept loop, so it is a report beside `State` at
      `crates/nvs-cli/src/service.rs:576` — `WATCHDOG_USEC` is the period and `WATCHDOG_PID` says
      whether this process may send it — gated on what the process's stall detector says, which is
      `nvs_host::Watchdog::with`'s sink over the `Watchdog::new()` at
      `crates/nvs-cli/src/serve.rs:509`. A ping from a thread that lives whether or not a core turns
      proves only that the process exists; if that gate cannot be built cheaply, take the line out of
      `unit()` instead and amend the rule, and say which in the commit.
- [ ] **A reload's outcome is not written to `Core\Log`** — `rule:config/one-local-control-socket`'s
      last sentence, and the reload it is about is `crate::control::Process::published` at
      `crates/nvs-cli/src/control.rs:117`. The `Report` it hands back is what an operator gets on
      stdout today and nothing else; the log line is the record a deployment reads afterwards.

## Backlog

- A drain does not cut a connection's idle wait short — `crates/nvs-server/src/io.rs:141`,
  `crates/nvs-server/src/socket.rs:266`; `nvs_host::wake_at_drain` is now the seam for both readers.
- A connection between one response and the next request waits under `header` and not `keepalive` —
  `crates/nvs-server/src/io.rs:130`'s `Phase::KeepAlive` looks never to be set.
- A `[[schedule]]` waits out one interval before a stop ends its ticker —
  `crates/nvs-cli/src/serve.rs`'s `keep_ticking` closure says so.
- Nothing sends a datagram in any test: the `Supervisor` sink is what the case drives, so
  `crates/nvs-cli/src/service.rs:709`'s `mod systemd` is compiled on Linux and never run.
- `[context] modules` printed neither `crates/nvs-runtime/src/drain.rs` nor
  `crates/nvs-server/src/io.rs` and `socket.rs`, which are the drain's other readers.
