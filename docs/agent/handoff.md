# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal
opens). **Stage 3 is now whole on both halves.** `nvs serve` binds the endpoint `[control] socket`
names before any listener is bound, runs `nvs_server::control::serve` on one thread of its own, and
supplies `crate::control::Process` as the `Controlled` behind it; `nvs ctl` drives all three
operations against it.

The order is `crates/nvs-cli/src/serve.rs`'s `bind_sockets`, and its doc owns why: the endpoint's
directory is held to `rule:config/ownership-is-the-trust-boundary`, so a boot that bound listeners
first would be answering requests on the way to refusing to start. A test passes a verdict instead
of the real check, which is what makes the order assertable on a platform whose filesystem cannot be
put in the refused state.

**A reload now reaches the next request.** `Serving` holds an `Arc<nvs_config::Current>` rather than
a snapshot (`Serving::live`; `Serving::new` wraps a tree nothing publishes into, which is every
embedder and every test), and `nvs serve` hands it the holder `crate::control::Process` publishes
into. The unit cache is re-keyed with it — `[[extension]]` is reloadable and is the whole of the
environment digest — so `Compiler::rekey` drops every unit keyed under the digest it leaves and the
report's `invalidated` count is a fact rather than a prediction.

Left in stage 3: the two cases its `-p nvs-cli` check names that are about **stopping** —
`a_terminating_signal_drains_serve_and_every_connection_closes_cleanly` and
`sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping`. Nothing installs a signal
handler and nothing writes to `NOTIFY_SOCKET` yet.

## Next group

**Stage 3: a signal drains `serve`, and systemd is told each state** — one file set:
`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/service.rs`.

- [ ] **A terminating signal begins the one drain** — installed in `run` beside the control thread
      at `crates/nvs-cli/src/serve.rs:411`, ending in `nvs_server::Draining::process().begin()`,
      which is the same state machine the SCM stop and the control endpoint's own loop already read
      (`rule:concurrency/a-drain-closes-a-connection-cleanly`). The accept loop's half is
      `crates/nvs-server/src/serve.rs:473`'s `Draining` and the bit under it is
      `crates/nvs-runtime/src/drain.rs:123`, whose `wake_at_drain` is what makes a parked connection
      see the drain rather than wait out its own idle timer.
- [ ] **`sd_notify` says `READY`, `RELOADING`, `READY`, `STOPPING`** — beside `unit()` at
      `crates/nvs-cli/src/service.rs:491`, which is where this binary already knows what a service
      manager was told about it (`rule:packaging/a-service-is-one-stored-argv`). The reload leg fires
      from `crate::control::Process::reload` at `crates/nvs-cli/src/control.rs:100`, which is the one
      function the socket, `ExecReload` and `PARAMCHANGE` all end in.
- [ ] **A reload's outcome is written to `Core\Log`** — `rule:config/one-local-control-socket`'s last
      sentence, unbuilt: `crate::control::Process::reload` at
      `crates/nvs-cli/src/control.rs:100` returns the report and logs nothing.

## Backlog
- `nvs ctl`'s wait for a server that answers nothing is unbounded — the endpoint serializes
  operations, so bounding it is a `--timeout` decision rather than a constant
  (`crates/nvs-cli/src/ctl.rs`'s `exchange`).
- On Windows a control client that stops *reading* can still wedge the thread: a `WriteFile` into a
  full pipe buffer waits, where the Unix socket answers `WouldBlock` and the idle bound catches it.
  A `config` listing is the only answer big enough to reach it (`crates/nvs-config/src/control.rs`).
- `nvs_config::control::Endpoint::bound()` has no caller now that `accept` is on the endpoint.
- `[opcache]`'s `validate`, `revalidate_freq` and the artifact cache are still read once at
  `Compiler::new`, so a reload that changes them is reported as applied and is not
  (`crates/nvs-cli/src/script.rs:288`).
- Stale prose — `docs/plan/m7.md`'s carrier list, `crates/nvs-cli/src/serve.rs`'s "no
  configuration" gap — belongs to goal `plan-truth`.
- The four `unowned` gaps at `crates/nvs-server/src/route.rs:30` and its siblings — goal
  `unowned-closures`.
