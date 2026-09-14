# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal
opens). **Stage 3's reporting half is complete**: a `Type=notify` unit gets `READY=1` once every
listener is bound, `RELOADING=1` and `READY=1` around every reload, `STOPPING=1` once the drain has
begun, and `WATCHDOG=1` for as long as the fleet is turning.

**The watchdog ping is gated rather than timed.** `crate::service::Heartbeat` sends `State::Alive`
every half of `WATCHDOG_USEC` — and only where `WATCHDOG_PID` admits this process — while
`nvs_host::Watchdog::turning()` holds, which is the stall detector read rather than written to, so
no request path pays for it. One wedged core of several keeps the ping; it stops only where no core
turns at all, because `rule:http-server/a-wedged-core-is-shed-never-killed` sheds the first and a
restart is the only recovery left for the second. Both rule fragments now say so, and `service.rs`'s
module doc § *The manager is told what state this process is in* points at `Heartbeat` for the rest.

Left in stage 3: a reload's outcome is still not written to `Core\Log`, and the door it needs does
not exist yet — the next group is that door and then the write.

## Next group

**Stage 3: a reload writes its outcome to `Core\Log`** — one file set:
`crates/nvs-runtime/src/ctx/output.rs`, `crates/nvs-cli/src/control.rs`.

- [ ] **The log write has no door for a thread that holds no request** — `Ctx::write_log_record` at
      `crates/nvs-runtime/src/ctx/output.rs:251` is `rule:errors/record-producers`'s one reader of
      `[log] target`, `level` and `format`, and `resolve_log_target` at
      `crates/nvs-runtime/src/ctx/output.rs:345` reads them off the context's own config. Give that
      resolution a `Ctx`-free entry — a config and an `nvs_render::Record` in, the rendered line out
      — so the control thread does not become a second reader of the directive, and decide there
      where such a record goes when `[log] target` names nothing, which for a writer that is no
      request cannot be `LogChannel::Output` at `crates/nvs-runtime/src/ctx/output.rs:108`.
- [ ] **A reload's outcome is not written to `Core\Log`** — `rule:config/one-local-control-socket`'s
      last sentence, over the reload at `crates/nvs-cli/src/control.rs:117`
      (`crate::control::Process::published`) and the `Reloading`/`Ready` pair wrapping it at
      `crates/nvs-cli/src/control.rs:146`. The `Report` it hands back is what an operator reads on
      `nvs ctl`'s stdout and nothing else; the record is what the deployment reads afterwards.
      `crates/nvs-stdlib/src/log.rs:305`'s `record` is the shape one is built in, and the level a
      refused reload carries is the half to decide first.

## Backlog

- A drain does not cut a connection's idle wait short — `crates/nvs-server/src/io.rs:141`,
  `crates/nvs-server/src/socket.rs:266`; `nvs_host::wake_at_drain` is now the seam for both readers.
- A connection between one response and the next request waits under `header` and not `keepalive` —
  `crates/nvs-server/src/io.rs:130`'s `Phase::KeepAlive` looks never to be set.
- A `[[schedule]]` waits out one interval before a stop ends its ticker —
  `crates/nvs-cli/src/serve.rs`'s `keep_ticking` closure says so.
- Nothing sends a datagram in any test: the `Supervisor` sink is what the case drives, so
  `crates/nvs-cli/src/service.rs`'s `mod systemd` is compiled on Linux and never run.
- Stage 4 is unwritten — `a_unix_socket_listen_entry_binds_with_its_mode_on_unix_and_is_refused_once_elsewhere`
  is the test the driver's acceptance check is waiting for.
- `[context] modules` printed neither `crates/nvs-host/src/watchdog.rs`, which this item's own gate
  lives in, nor `crates/nvs-runtime/src/drain.rs` and `crates/nvs-server/src/io.rs`/`socket.rs`,
  which are the drain's other readers.
